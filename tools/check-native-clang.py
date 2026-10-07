"""Compile changed native C for arm64 with the installed LLVM frontend.

No SDK/tool downloads or game bytes. Minimal CRT declarations are compile-only;
this gate supplements, and does not replace, the Apple clang CI build.
"""
from pathlib import Path
import argparse
import subprocess
from prepare_sdk import prepare


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--compiler',required=True)
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[1]
    output=root/'target/core-native-clang'
    prepare(root/'game/decomp',output)
    sources=[root/'port/runtime.c',root/'port/gameplay.c',root/'port/map.c',root/'port/title_services.c',
             output/'gameplay_consumers.c',output/'map0_s00.c',output/'map_info.c',output/'player_spawn.c']
    flags=['-target','aarch64-apple-ios15.0','-ffreestanding','-std=c11','-Wall','-Wextra','-Werror','-nostdinc']
    for include in [root/'tools/layout-include',root/'port',root/'port/include',output,root/'game/decomp/include',root/'game/decomp/src/main']:
        flags+=['-I',str(include)]
    for name in ['ccos','csin','csqrt','catan']:
        flags.append('-fno-builtin-'+name)
    failed=0
    for source in sources:
        result=subprocess.run([args.compiler,str(source),'--checks=-*,clang-analyzer-core.DivideZero','--warnings-as-errors=*','--',*flags],capture_output=True,text=True)
        print(('PASS' if result.returncode==0 else 'FAIL')+': '+source.name,flush=True)
        if result.returncode:
            print(result.stdout+result.stderr,flush=True)
            failed+=1
    print(f'Native arm64 C frontend: {len(sources)-failed} passed, {failed} failed. Apple SDK/runtime not exercised.')
    return int(failed!=0)


if __name__=='__main__':
    raise SystemExit(main())
