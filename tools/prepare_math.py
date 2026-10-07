"""Compile the pinned GPL libkmath kernels to native straight-line C.

SPDX-License-Identifier: GPL-3.0-only. SDK tables come from the reference SDK,
never the player's disc. This is build-time translation, not code execution.
"""
from pathlib import Path
import re
import struct

NOTICE = '/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n'


def elf_data(path):
    blob = path.read_bytes()
    assert blob[:6] == b'\x7fELF\x01\x01', 'pinned little-endian ELF32 SDK'
    offset, = struct.unpack_from('<I', blob, 32)
    stride, count, names_index = struct.unpack_from('<HHH', blob, 46)
    sections = [struct.unpack_from('<10I', blob, offset + i * stride) for i in range(count)]
    names = sections[names_index]
    strings = blob[names[4]:names[4]+names[5]]
    for section in sections:
        name = strings[section[0]:].split(b'\0')[0]
        if name == b'.data':
            return blob[section[4]:section[4]+section[5]]
    raise ValueError('missing pinned SDK data section')


def generate(decomp, out):
    atan = struct.unpack('<1025h', elf_data(decomp/'lib/libgte/ratan.o')[:2050])
    sqrt = struct.unpack('<192h', elf_data(decomp/'lib/libgte/sqrtbl.o'))
    inverse = struct.unpack('<192h', elf_data(decomp/'lib/libgte/msc02.o')[20:404])
    source = [NOTICE, '#include "gameplay.h"\n',
        'static const s16 atan_table[1025]={' + ','.join(map(str, atan)) + '};\n',
        'static const s16 sqrt_table[192]={' + ','.join(map(str, sqrt)) + '};\n',
        'static const s16 inverse_sqrt_table[192]={' + ','.join(map(str, inverse)) + '};\n',
        '''// PORT: Preserve SDK ratio scaling, octants and table quantization.
s32 ratan2(s32 y,s32 x) {
    bool negative_x=x<0,negative_y=y<0;
    u32 ax=negative_x?0u-(u32)x:(u32)x,ay=negative_y?0u-(u32)y:(u32)y;
    if(!ax && !ay)return 0;
    u32 low=ay<ax?ay:ax,high=ay<ax?ax:ay;
    u32 index=(low&0x7fe00000u)?low/(high>>10):(low<<10)/high;
    if(index>1024)port_unimplemented("SDK atan ratio bounds");
    s32 angle=atan_table[index];
    if(ay>=ax)angle=1024-angle;
    if(negative_x)angle=2048-angle;
    return negative_y?-angle:angle;
}
s32 SquareRoot0(s32 value) {
    if(!value)return 0;
    // PORT: The SDK obtains LZCS/LZCR through COP2, including signed input.
    u32 leading=(u32)Lzc(value),even=leading&~1u;
    u32 normalized=even>=24?(u32)value<<(even-24):(u32)((s32)value>>(24-even));
    if(normalized<64 || normalized>=256)port_unimplemented("SDK sqrt table bounds");
    return (s32)(((u32)sqrt_table[normalized-64]<<((31-even)/2))>>12);
}
s32 VectorNormal(VECTOR* input,VECTOR* output) {
    u32 components[3]={(u32)input->vx,(u32)input->vy,(u32)input->vz};
    for(u32 i=0;i<3;i++)port_gte_write_data(9+i,components[i]);
    (void)port_gte_execute(0xA00428);
    u32 sum=port_gte_read_data(25)+port_gte_read_data(26)+port_gte_read_data(27);
    u32 even=(u32)Lzc((s32)sum)&~1u;
    u32 normalized=even>=24?sum<<((even-24)&31):(u32)((s32)sum>>((24-even)&31));
    if(normalized<64 || normalized>=256)port_unimplemented("SDK vector normalization bounds");
    port_gte_write_data(8,(u32)(s32)inverse_sqrt_table[normalized-64]);
    for(u32 i=0;i<3;i++)port_gte_write_data(9+i,components[i]);
    (void)port_gte_execute(0x190003D);
    u32 shift=((31-even)/2)&31;
    output->vx=(s32)port_gte_read_data(25)>>shift;
    output->vy=(s32)port_gte_read_data(26)>>shift;
    output->vz=(s32)port_gte_read_data(27)>>shift;
    return (s32)sum;
}
typedef struct {u32 v0,v1,a0,a1,a3,t0,t1,t2,t3,t4,t5,t6,t7,t8,t9;u64 product;} MathRegisters;
''']
    original = (decomp/'src/bodyprog/libkmath/libkmath.s').read_text()
    for name in ['Math_RotMatrix0','Math_RotMatrix1','Math_RotMatrixGte']:
        block = original.split('glabel '+name+'\n',1)[1].split('endlabel '+name,1)[0]
        lines = ['static void '+name+'Native(MathRegisters* r) {\n']
        def val(token):
            if token == '$zero': return '0u'
            return 'r->'+token[1:] if token.startswith('$') else token
        for op, arguments in re.findall(r'/\*[^*]+\*/[ \t]+(\w+)[ \t]*([^\n]*)',block):
            args = [x.strip() for x in arguments.split('#')[0].split(',')]
            if op in ['nop','jr']:
                continue
            if '%hi(g_SineTable)' in arguments or '%lo(g_SineTable)' in arguments:
                lines.append(val(args[0])+'=0;\n')
            elif op == 'lh':
                offset, register = re.fullmatch(r'([^()]+)\((\$\w+)\)',args[1]).groups()
                lines.append(val(args[0])+f'=(u32)Math_Sin((s32)(({val(register)}+({offset}))/2));\n')
            elif op in ['andi','or','addu','subu','addiu']:
                operator={'andi':'&','or':'|','addu':'+','subu':'-','addiu':'+'}[op]
                lines.append(f'{val(args[0])}={val(args[1])}{operator}(u32)({val(args[2])});\n')
            elif op in ['sll','srl','sra']:
                left=val(args[1])
                if op=='sra': left='(s32)'+left
                lines.append(f'{val(args[0])}=(u32)({left}{"<<" if op=="sll" else ">>"}{args[2]});\n')
            elif op=='negu': lines.append(f'{val(args[0])}=0u-{val(args[1])};\n')
            elif op=='mult': lines.append(f'r->product=(u64)((s64)(s32){val(args[0])}*(s32){val(args[1])});\n')
            elif op in ['mflo','mfhi']:
                lines.append(f'{val(args[0])}=(u32)(r->product{">>32" if op=="mfhi" else ""});\n')
            elif op in ['mtc2','mfc2']:
                register=int(args[1].split()[0][1:])
                lines.append(f'port_gte_write_data({register},{val(args[0])});\n' if op=='mtc2' else f'{val(args[0])}=port_gte_read_data({register});\n')
            elif op in ['gpl','gpf']:
                lines.append(f'(void)port_gte_execute({"0xA8003E" if op=="gpl" else "0x98003D"}u);\n')
            else: raise ValueError(f'unsupported pinned math instruction {name}: {op}')
        source.extend(lines+['}\n'])
    source.append('''void Math_RotMatrixZxyNegGte(const SVECTOR* rot,MATRIX* mat) {
    MathRegisters r={0};r.v0=0u-(u32)(s32)rot->vz;r.v1=0u-(u32)(s32)rot->vx;r.a3=0u-(u32)(s32)rot->vy;
    Math_RotMatrixGteNative(&r);
    mat->m[2][0]=(s16)r.t0;mat->m[0][0]=(s16)r.t1;mat->m[2][1]=(s16)r.t2;
    mat->m[0][1]=(s16)r.t3;mat->m[2][2]=(s16)r.t4;mat->m[0][2]=(s16)r.t5;
    mat->m[1][0]=(s16)r.t6;mat->m[1][1]=(s16)r.t7;mat->m[1][2]=(s16)r.t8;
}
void Math_RotMatrixZxyNeg(const SVECTOR* rot,MATRIX* mat) {
    MathRegisters r={0};r.v0=0u-(u32)(s32)rot->vz;r.v1=0u-(u32)(s32)rot->vx;r.a3=0u-(u32)(s32)rot->vy;
    Math_RotMatrix0Native(&r);
    mat->m[1][2]=(s16)r.t0;mat->m[2][0]=(s16)r.t1;mat->m[0][0]=(s16)r.t2;mat->m[2][1]=(s16)r.t3;mat->m[0][1]=(s16)r.t4;
    Math_RotMatrix1Native(&r);
    mat->m[2][2]=(s16)r.t0;mat->m[0][2]=(s16)r.t1;mat->m[1][0]=(s16)r.t2;mat->m[1][1]=(s16)r.t3;
}
''')
    (out/'native_math.c').write_text(''.join(source))
