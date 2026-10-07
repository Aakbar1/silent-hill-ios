"""Lower pinned PsyQ COP2 inline macros into native C register calls.

SPDX-License-Identifier: GPL-3.0-only. No game bytes or game disassembly.
This is build-time translation of SDK straight-line helpers, not a CPU runner.
Unknown instructions/operands fail the build rather than silently disappearing.
"""
from pathlib import Path
import argparse
import re


def macros(text):
    return re.findall(r"^#define\s+(\w+)\s*\(([^)]*)\)(.*?)(?=^#|\Z)",
                      text.replace("\\\n", " "), re.M | re.S)


def generate(decomp, out):
    original = (decomp / "include/psyq/inline_c.h").read_text()
    overrides = dict((n, b) for n, _, b in macros((decomp / "include/inline_no_dmpsx.h").read_text()))
    output = [original[:original.index("/*\n * Type 1")],
              "// PORT: Native register transfers generated from pinned SDK helpers.\n",
              "#ifndef SH_INLINE_C_H\n#define SH_INLINE_C_H\n"]
    count = 0
    for name, arguments, body in macros(original):
        if "__asm__" not in body:
            if name in ("gte_mvmva", "gte_mvmva_b"):
                output.append(f"#define {name}(sf,mx,v,cv,lm) ((void)port_gte_execute(0x12u | ((u32)(sf)<<19) | ((u32)(mx)<<17) | ((u32)(v)<<15) | ((u32)(cv)<<13) | ((u32)(lm)<<10)))\n")
            continue
        args = [a.strip() for a in arguments.split(",") if a.strip()]
        if name in ("gte_mvmva_core", "gte_mvmva_core_b"):
            output.append(f"#define {name}(r0) ((void)port_gte_execute((u32)(r0)))\n")
            continue
        # _b is the same operation with CPU delay nops omitted.
        command_body = overrides.get(name.removesuffix("_b"), body)
        code_strings = re.findall(r'"([^"\n]*)"', command_body.split(":")[0])
        instructions = [s.strip() for s in "".join(code_strings).split(";") if s.strip()]
        used = sorted(set(re.findall(r"\$(\d+)", " ".join(instructions))))
        lines = [f"u32 sh_gte_r{r}=0;" for r in used if r != "0"]
        def value(token):
            token = token.strip()
            if token.startswith("$"):
                return "0u" if token == "$0" else "sh_gte_r" + token[1:]
            if token.startswith("%"):
                return f"(u32)({args[int(token[1:])]})"
            return token
        def address(token):
            match = re.fullmatch(r"(-?\d+)\(\s*%(\d+)\s*\)", token.strip())
            if not match:
                raise ValueError(f"{name}: unsupported address {token}")
            return f"((u8*)({args[int(match[2])]})+({match[1]}))"
        for instruction in instructions:
            parts = instruction.split(None, 1)
            op = parts[0]
            operands = [p.strip() for p in parts[1].split(",")] if len(parts) > 1 else []
            if op == "nop":
                continue
            if op == ".word":
                if operands[0].startswith("%"):
                    # dmpsx's macro parameters occupy different bits from COP2.
                    raise ValueError(f"unexpected dynamic command {name}")
                lines.append(f"(void)port_gte_execute({operands[0]}u);")
            elif op in ("lwc2", "swc2"):
                reg = int(operands[0][1:])
                addr = address(operands[1])
                if op == "lwc2":
                    lines.append(f"port_gte_write_data({reg},port_gte_load32({addr}));")
                elif reg in (12, 13, 14, 15):
                    lines.append(f"port_gte_store_sxy({addr},{reg});")
                else:
                    lines.append(f"port_gte_store32({addr},port_gte_read_data({reg}));")
            elif op in ("ctc2", "mtc2"):
                fn = "control" if op == "ctc2" else "data"
                lines.append(f"port_gte_write_{fn}({int(operands[1][1:])},{value(operands[0])});")
            elif op in ("cfc2", "mfc2"):
                fn = "control" if op == "cfc2" else "data"
                lines.append(f"{value(operands[0])}=port_gte_read_{fn}({int(operands[1][1:])});")
            elif op in ("lw", "lh", "lhu", "lbu"):
                addr = address(operands[1])
                expr = {"lw": f"port_gte_load32({addr})", "lh": f"(u32)(s32)(s16)port_gte_load16({addr})",
                        "lhu": f"port_gte_load16({addr})", "lbu": f"*(const u8*){addr}"}[op]
                lines.append(f"{value(operands[0])}={expr};")
            elif op in ("sw", "sh", "sb"):
                addr, val = address(operands[1]), value(operands[0])
                lines.append(f"*(u8*){addr}=(u8)({val});" if op == "sb" else f"port_gte_store{32 if op=='sw' else 16}({addr},{val});")
            elif op in ("sll", "srl", "sra", "addu", "subu", "addi", "or", "and"):
                dest, a, b = map(value, operands)
                if op == "sra":
                    expr = f"port_gte_asr({a},{b})"
                elif op in ("sll", "srl"):
                    expr = f"({a}) {'<<' if op=='sll' else '>>'} (({b})&31)"
                else:
                    expr = f"({a}) {dict(addu='+',subu='-',addi='+',or_='|',and_='&').get(op, '|' if op=='or' else '&')} ({b})"
                lines.append(f"{dest}={expr};")
            else:
                raise ValueError(f"{name}: unsupported instruction {instruction}")
        for r in used:
            if r != "0":
                lines.append(f"(void)sh_gte_r{r};")
        output.append(f"#define {name}({arguments}) do {{ {' '.join(lines)} }} while (0)\n")
        count += 1
    output.append("#endif\n")
    (out / "psyq/inline_c.h").write_text("".join(output))
    (out / "inline_no_dmpsx.h").write_text('#include "gte_native.h"\n')
    (out / "psyq/gtemac.h").write_text((decomp / "include/psyq/gtemac.h").read_text())
    commands = [(n, int(re.search(r'\.word (0x[0-9A-Fa-f]+)', b)[1], 16))
                for n, b in overrides.items() if re.search(r'\.word (0x[0-9A-Fa-f]+)', b)]
    probe = ['/* SPDX-License-Identifier: GPL-3.0-only */\n#include "boot.h"\n',
             'int port_gte_command_probe(u32 index,u32* words) { switch(index) {\n']
    for index, (name, _) in enumerate(commands):
        probe.append(f'case {index}: {name}();break;\n')
    probe.append('default: return 1; } for(u32 i=0;i<32;i++) {words[i]=port_gte_read_data(i);words[i+32]=port_gte_read_control(i);} return 0;}\n')
    (out / 'gte_command_probe.c').write_text(''.join(probe))
    (out / 'gte_commands.rs').write_text('const COMMANDS: &[u32] = &[' + ','.join(hex(op) for _, op in commands) + '];\n')
    print(f"Native PsyQ GTE: {count} straight-line macros")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--decomp", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    generate(args.decomp, args.out)
