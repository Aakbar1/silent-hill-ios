"""Prepare pinned GPL world rendering C, never reading the player's disc.

SPDX-License-Identifier: GPL-3.0-only. Native sidecars preserve the parallel
player lane's records. Only calls to its explicit rendering guards are rebound.
"""
from pathlib import Path
import argparse
import re
import subprocess
from prepare_gameplay import between, function
from prepare_camera import narrow, NOTICE
from prepare_maps import initializer

REBIND = {
    'Gfx_LoadScreenMapEffectsUpdate': 'port_render_load_effects',
    'Gfx_EffectsUpdate': 'port_render_effects',
    'WorldGfx_CharaDraw': 'port_render_character',
    'Screen_BackgroundMotionBlur': 'port_render_motion_blur',
}


RUST_TESTS = r'''
// SPDX-License-Identifier: GPL-3.0-only
#[cfg(test)]
mod render_milestone {
    use super::*;

    #[test]
    fn native_render_bounds_final_triples_and_light_basis() {
        unsafe extern "C" {
            fn port_render_leaf_probe(out: *mut u32) -> i32;
        }
        let mut out = [0; 13];
        // SAFETY: Synthetic leaves and scratch are local; COP2 state is thread-local.
        assert_eq!(unsafe { port_render_leaf_probe(out.as_mut_ptr()) }, 1);
        assert_eq!(&out[..6], &[0, 0xffe0_0020, 128, 256, 0xa5a5_a5a5, 0xa5a5]);
        assert_eq!(&out[6..8], &[254, 0xa5a5_a5a5]);
        assert_eq!(
            &out[8..],
            &[0x0004_0001, 0x0002_0007, 0x0008_0005, 0x0006_0003, 9]
        );
    }

    #[test]
    #[ignore = "requires the owned US 1.1 disc and a wgpu adapter; writes a private capture"]
    fn first_map_view() {
        unsafe extern "C" {
            fn port_capture_render_boundary() -> i32;
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let private = root
            .join("../../private")
            .canonicalize()
            .expect("private root");
        let disc = GameDisc::open(private.join("disc/Silent Hill (USA).bin"))
            .expect("verified owned disc");
        let directory = private.join("work/world");
        std::fs::create_dir_all(&directory).expect("private captures");
        let replay = ReplayPad::parse(
            &std::fs::read_to_string(root.join("docs/core/replays/new-game-opening-skip.txt"))
                .expect("original opening replay"),
        )
        .expect("opening replay rows");
        let mut options = crate::gpu_wgpu::Options::default();
        if let Ok(renderer) = std::env::var("SH_RENDER_TEST_BACKEND") {
            options.renderer = match renderer.as_str() {
                "soft" => crate::gpu_wgpu::RendererKind::Soft,
                "wgpu" => crate::gpu_wgpu::RendererKind::Wgpu,
                _ => panic!("SH_RENDER_TEST_BACKEND must be soft or wgpu"),
            };
        }
        if let Ok(scale) = std::env::var("SH_RENDER_TEST_SCALE") {
            options.scale = scale.parse().expect("SH_RENDER_TEST_SCALE integer");
        }
        crate::gpu_wgpu::configure(options).expect("render diagnostic options");
        let capture = match options.renderer {
            crate::gpu_wgpu::RendererKind::Soft => "first_map_view-soft.png".to_owned(),
            crate::gpu_wgpu::RendererKind::Wgpu if options.scale == 4 => {
                "first_map_view.png".to_owned()
            }
            crate::gpu_wgpu::RendererKind::Wgpu => format!("first_map_view-{}x.png", options.scale),
        };
        let gpu = crate::gpu_wgpu::backend().expect("configured renderer");
        let now = Instant::now();
        HOST.with(|cell| {
            *cell.borrow_mut() = Some(Host {
                disc,
                gpu,
                spu: Box::new(crate::backend::SilentSpu::default()),
                pad: Box::new(replay),
                assets: AssetStore::default(),
                saves: SaveStore::app_data().expect("save store"),
                proxy: None,
                frames: 0,
                limit: 2300,
                screenshot: None,
                start: now,
                next: now,
                cancel: Arc::new(AtomicBool::new(false)),
                error: None,
                first_logo: None,
                last_frame: None,
                movie: None,
                state: 0,
                step: 0,
                movie_frames: 0,
                movie_skips: 0,
            });
        });
        // SAFETY: This isolated opt-in test is the sole C worker; jumps remain in C.
        let stopped = unsafe { port_run_game() };
        assert_eq!(stopped, 3, "retain the unlinked player startup guard");
        host(|h| {
            assert_eq!((h.frames, h.state, h.step), (2207, 10, 5));
            assert_eq!((h.movie_frames, h.movie_skips), (154, 2));
        });
        // SAFETY: The original game returned before the rendering diagnostic starts.
        assert_eq!(unsafe { port_capture_render_boundary() }, 0);
        host(|h| {
            assert!(h.error.is_none(), "host error: {:?}", h.error);
            let (w, height, pixels) = h.last_frame.as_ref().expect("map frame");
            assert_eq!((*w, *height), (320, 224));
            let colors: std::collections::HashSet<_> = pixels.iter().copied().collect();
            assert!(
                colors.len() > 64,
                "map needs geometry, not a flat fog background"
            );
            save_png(&directory.join(&capture), *w, *height, pixels).expect("private map capture");
            println!(
                "FIRST_MAP_VIEW PASS colors={} output={}",
                colors.len(),
                directory.display()
            );
        });
        HOST.with(|cell| *cell.borrow_mut() = None);
    }
}
'''


def generate(decomp, out):
    def read(path): return (decomp / path).read_text()
    game = read('include/game.h')
    effects = between(game, '/** @brief Map effects info.', '/** @brief Main system workspace.')
    effects = effects.replace('STATIC_ASSERT_SIZEOF(s_SysWork_2388, 392);',
        '// PORT: Native transition data pointer changes the work-record ABI.\nSTATIC_ASSERT_SIZEOF(s_SysWork_2388, 400);')
    body = read('include/bodyprog/bodyprog.h')
    scratch = between(body, 'typedef struct _GteScratchData', 'typedef struct\n{\n    /* 0x0 */ s8  field_0;')
    scratch2 = between(body, 'typedef struct\n{\n    /* 0x0   */ DVECTOR  screenXy_0', '// Something for inventory items.')
    # PORT: The upstream scratch declarations overlap arrays (HERO has 57
    # normals; an opening road mesh has 94 vertices). These are native working
    # fields, never wire layouts: give each byte-indexed arena 256 owned slots
    # and unite the two views by their names, preserving the matrix unions.
    scratch=re.sub(r'(screenXy_0|screenZ_168|field_18C|field_252|field_2B8)\[\d+\]',r'\1[256]',scratch)
    scratch=scratch.replace('                union', '    s32 field_21C[256];\n                union',1)
    extra=scratch2[scratch2.rindex('                 union'):scratch2.rindex('} s_GteScratchData2;')]
    scratch=scratch.replace('} s_GteScratchData;',extra+'} s_GteScratchData;\ntypedef s_GteScratchData s_GteScratchData2;\n')
    scratch=re.sub(r'/\* 0x[^*]*\*/','',scratch)
    scratch += 'STATIC_ASSERT_SIZEOF(s_GteScratchData,3732);\n_Static_assert(offsetof(s_GteScratchData,screenZ_168)==1024,"native render depths");\n_Static_assert(offsetof(s_GteScratchData,field_21C)==2560,"native render colors");\n'
    enums = between(body, 'typedef enum _PrimitiveType', 'typedef enum _LoadingScreenId')
    # Enums containing the named special environment flags are pointer-free.
    enums += between(game, 'typedef enum _SpecialEnvEventFlags', 'typedef enum _UnkGfxEnum')
    header = NOTICE + '#ifndef SH_RENDER_GENERATED_H\n#define SH_RENDER_GENERATED_H\n#include "world.h"\n' + enums + effects + scratch
    header += '\nenum {UnkGfxEnum_1=1,UnkGfxEnum_2=2};\n'
    header += 'typedef struct {u8 presetIdx0,presetIdx1;} s_MapEnvPresetIdxs;\n'
    header += between(body,'typedef struct\n{\n    /* 0x0 */ s_800AE204*','} s_800AE4DC;')+'} s_800AE4DC;\nSTATIC_ASSERT_SIZEOF(s_800AE4DC,16);\n'
    header += 'extern s_SysWork_2388 port_render_env;\nextern s_MapEffectsInfo MAP_EFFECTS_INFOS[21];\nextern s32 D_800AE1C0[];\n'
    header += 'void SetPriority(void*,s32,s32);void port_render_world_objects(s_WorldGfxWork*);void port_render_held_item(void);\n'
    sources = []
    selections = {
        'src/bodyprog/world/world_effects.c': [
            'WorldEnv_MapPresetSet','Gfx_MapEnvSet','func_8003EDA8','func_8003EDB8',
            'Gfx_LoadScreenMapEffectsUpdate','Gfx_MapEnvUpdate','Gfx_MapEnvStepUpdate',
            'Gfx_FogParametersSet','Gfx_EffectsUpdate','func_8003F4DC','func_8003F654',
            'func_8003F6F0','Math_WeightedAverageGet','func_8003F838','func_8003FCB0',
            'func_8003FD38','func_8003FE04','func_8003FEC0','WorldEnv_FogLightingParamsUpdate'],
        'src/bodyprog/gfx/bodyprog_80055028.c': [
            'WorldGfx_2dEffectsDraw','WorldEnv_WorldLightingParamSet','WorldEnv_FogParamsSet',
            'WorldEnv_WorldLightTintSet','WorldEnv_LightPositionGet','WorldEnv_LightRotationAndIntensityGet',
            'WorldEnv_LightDirectionAndIntensityGet','Gfx_FlashlightPositionUpdate','func_80055648',
            'func_800557DC','func_80055814','func_800559A8','func_80055A50','func_80055A90',
            'func_80055B74','func_80055C3C','func_80055D78','func_80055E90','func_80055ECC','func_80055F08'],
        'src/bodyprog/gfx/bodyprog_80056D8C.c': re.findall(
            r'^void\s+(\w+)\(', read('src/bodyprog/gfx/bodyprog_80056D8C.c'), re.M) + ['func_800571D0','func_8005AA08'],
        'src/bodyprog/world/bodyprog_bone_80044F14.c': ['func_80045534'],
        'src/bodyprog/world/world_draw.c': ['WorldGfx_CharaDraw','WorldGfx_CharaClutYGet'],
        'src/bodyprog/screen/background_draw.c': ['Screen_BackgroundMotionBlur'],
        'src/bodyprog/gfx/billboard_draw.c': ['Gfx_BillboardDraw'],
    }
    for path, names in selections.items():
        for name in names:
            code = function(read(path), name).removeprefix('static ')
            code = code.replace('g_SysWork.field_2388', 'port_render_env')
            if name=='Gfx_BillboardDraw':
                code=code.replace('g_ViewCoord','vwGetViewCoord()')
                code=code.replace('func_8005A478(&sp90,','func_8005A478((s_GteScratchData*)&sp90,').replace('func_8005A838(&sp90,','func_8005A838((s_GteScratchData*)&sp90,')
                code=code.replace('RotTransPers(&curPtr->position, &sxy,', 'RotTransPers(&curPtr->position, (s32*)&sxy,')
                code=code.replace('poly_gt4->tpage = tPage;', 'poly_gt4->tpage = (u16)tPage;').replace('poly_gt4->clut       = clut;', 'poly_gt4->clut       = (u16)clut;')
                code=re.sub(r'    // @hack Make sure compiler.*?    if \(g_WorldEnvWork.field_0', '    if (g_WorldEnvWork.field_0',code,flags=re.S)
                code=re.sub(r'^\s*s32\s+i;\n','\n',code,flags=re.M)
            # PORT: The loading player is drawn before WorldGfx_MapInit publishes
            # the streaming workspace's mapInfo. Use the already active native
            # descriptor for its water-zone data during that interval.
            code=code.replace('g_WorldGfxWork.mapInfo->waterZones',
                '(g_WorldGfxWork.mapInfo ? g_WorldGfxWork.mapInfo : g_MapOverlayHdr.mapInfo)->waterZones')
            code = code.replace('WorldEnv_WorldLightTintParamSet(', 'port_render_light_tint(')
            code = code.replace('LoadAverageCol(', 'port_render_average_col(')
            if name=='Gfx_FlashlightPositionUpdate':code=code.replace('s_WaterZone* waterZones)', 'const s_WaterZone* waterZones)')
            code=re.sub(r'(g_WorldEnvWork.waterZones\s*=)\s*waterZones;',r'\1 (s_WaterZone*)waterZones;',code)
            if name=='func_80057A3C':code=code.replace('SVECTOR3*', 'SVECTOR*')
            if name=='func_8005A838':code=code.replace('{\n','{\n    (void)scratchData;\n',1)
            if name=='WorldGfx_CharaClutYGet':code=code.replace('{\n','{\n    (void)charaId;\n',1)
            if name=='func_8005AC50':
                for unused in ['temp_v1','temp_t4_2','temp_v0','temp_v0_2','var_v0','var_v1','localOt','var_t5']:
                    code=re.sub(r'^\s*(?:s16|s32|GsOT_TAG\*|u16\*)\s+'+unused+r';\n','\n',code,flags=re.M)
                code=re.sub(r'scratchData->screenZ_168\[([^]]+)\]',r'port_render_depth(scratchData,\1)',code)
                code=re.sub(r'\*\(s32\*\)&scratchData->field_21C\[([^]]+)\]',r'port_render_normal_color(scratchData,\1)',code)
            code = code.replace('const s_MapEffectsInfo* arg0,', 's_MapEffectsInfo* arg0,') if name=='func_8003FE04' else code
            code = code.replace('s_StructUnk3* envSettings0, s_StructUnk3* envSettings1', 'const s_StructUnk3* envSettings0, const s_StructUnk3* envSettings1')
            code = code.replace('SVECTOR3* arg3, SVECTOR* arg4', 'SVECTOR* arg3, SVECTOR* arg4')
            if name=='func_80057A3C':code=code.replace('SVECTOR3* arg3)', 'SVECTOR* arg3)')
            code = code.replace('ApplyMatrixLV(&viewMat, &g_SysWork.lightPosition,', 'ApplyMatrixLV(&viewMat, (VECTOR*)&g_SysWork.lightPosition,')
            if name == 'Gfx_EffectsUpdate':
                code = code.replace('{\n', '{\n    port_render_env.isFlashlightOn = g_SysWork.field_2388.isFlashlightOn;\n', 1)
            code = code.replace('WorldObjects_DrawAllObjects(', 'port_render_world_objects(')
            code = code.replace('WorldGfx_HeldItemDraw(', 'port_render_held_item(')
            code = code.replace('CHARA_FILE_INFOS[charaId].field_6', 'CHARA_FILE_INFOS[charaId].cameraOffsetY')
            # PORT: Harry's optional extra bounds array is NULL in the upstream table.
            code = code.replace('CHARA_FILE_INFOS[charaId].field_8', 'NULL')
            if name == 'WorldGfx_CharaDraw':
                code = code.replace('    timer = CLAMP', '    if(charaId!=Chara_Harry)port_unimplemented("non-Harry character rendering metadata");\n    timer = CLAMP')
            code = code.replace('(GsOT*)&g_OtTags1[g_ActiveBufferIdx + 1][0]',
                '(GsOT*)&g_OtTags1[g_ActiveBufferIdx][ORDERING_TABLE_SIZE-1]')
            code = code.replace('scratchData = PSX_SCRATCH_ADDR(0);', 'scratchData = &port_render_scratch;')
            # PORT: Native decoded leaf arrays do not provide the PS1 word-overread padding.
            if name == 'func_800574D4':
                start = code.index('    while (var_a2')
                end = code.index('    unkPtr     =', start)
                code = code[:start] + '''    for(s32 i=0;i<meshHdr->vertexCount;i++) {
        screenXy[i]=vertexXy[i];var_a2[i]=vertexZ[i];
    }
''' + code[end:]
                code = code.replace('        *(u32*)unkPtrDest = *(u32*)unkPtr;\n        unkPtr += 4;\n        unkPtrDest += 4;', '        *unkPtrDest++ = *unkPtr++;')
            if name == 'func_8005759C':
                code = code.replace('screenXyPtr  = &scratchData->screenXy_0[vertOffset];', 'screenXyPtr  = (s32*)&scratchData->screenXy_0[vertOffset];')
                code = code.replace('vertexXyPtr  = meshHdr->verticesXy;', 'vertexXyPtr  = (s32*)meshHdr->verticesXy;')
                code = code.replace('vertexXyPtr < &meshHdr->verticesXy[meshHdr->vertexCount]', 'vertexXyPtr < (s32*)&meshHdr->verticesXy[meshHdr->vertexCount]')
            if name=='func_80057090':code=code.replace('    modelHdr = modelInfo->modelHdr;', '    modelHdr = modelInfo->modelHdr;\n    port_render_model_check(modelHdr);')
            if name=='func_8005A900':
                # PORT: SDK triples must not overread native exact-size leaves.
                code=code.split('{',1)[0]+'{\n    port_render_project(meshHdr,offset,scratchData,viewMat);\n}\n'
            if name=='func_8005AA08':
                # PORT: Same NCT/three-color groups, bounded final native triple.
                code=code.split('{',1)[0]+'{\n    return port_render_normals(meshHdr,arg1,scratchData);\n}\n'
            code = code.replace('func_8005AA08(curMeshHdr, normalOffset, scratchData)', 'func_8005AA08(curMeshHdr, normalOffset, (s_GteScratchData2*)scratchData)')
            # PORT: Generic field_N names belong to unrelated records of
            # different widths. Do not infer their type across record scopes.
            view=re.sub(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(field_\w+)\s*;',
                r'\1 port_hidden_\2;',header + read('include/bodyprog/gfx/world.h'))
            code = narrow(code, view)
            code = code.replace('port_render_env.field_4 = (u8)(primData)', 'port_render_env.field_4 = (s8*)primData')
            code = re.sub(r'^(\s*\w+(?:(?:->|\.)\w+|\[[^]]+\])*(?:->|\.)[rgb][0-3]?\s*=(?!=))\s*([^;]+);',r'\1 (u8)(\2);',code,flags=re.M)
            code = re.sub(r'(flags\.field_00\[[^]]+\]\s*=(?!=))\s*([^;]+);',r'\1 (u8)(\2);',code)
            code = re.sub(r'(effectsInfo\.field_E\s*=(?!=))\s*([^;]+);',r'\1 (u8)(\2);',code)
            code = re.sub(r'(rotMatrix_3E4\[[^]]+\]\[[^]]+\]\s*=)\s*([^;]+);',r'\1 (s16)(\2);',code)
            code = re.sub(r'(temp_a2\[2\]\s*=)\s*([^;]+);',r'\1 (s16)(\2);',code)
            code = code.replace('*var_a3 = var_v1;', '*var_a3 = (u8)var_v1;').replace('*(var_t0 - 1) = var_a1;', '*(var_t0 - 1) = (u8)var_a1;')
            code = code.replace('void*     endPtr;', 'u8*       endPtr;')
            code = code.replace('endPtr = (void*)', 'endPtr = (u8*)')
            code = re.sub(r'(GsOUT_PACKET_P\s*=)\s*([^;]+);',r'\1 (PACKET*)(\2);',code)
            code = re.sub(r'\breturn ([^;]+);',r'return (u8)(\1);',code) if code.startswith('u8 ') else code
            # PORT: Preserve the original unsigned screen-bound comparisons.
            code=re.sub(r'(\(s16\)\w+ \+ \w+) (<|>=) (temp_t[01]\w*)',r'(u32)(\1) \2 \3',code)
            code=re.sub(r'(\(\(s16\)\w+ \+ \w+\)) (<|>=) (temp_t[01]\w*)',r'(u32)\1 \2 \3',code)
            code = code.replace('func_8005AC50(curMeshHdr, scratchData,', 'func_8005AC50(curMeshHdr, (s_GteScratchData2*)scratchData,')
            code = re.sub(r'\bpoly3\s*=\s*(\(PACKET\*\)[^;]+);',r'poly3 = (POLY_GT4*)(\1);',code)
            code = re.sub(r'\bsprt\s*=\s*(\(PACKET\*\)[^;]+);',r'sprt = (SPRT*)(\1);',code)
            code = code.replace('Q8_TO_Q12(CHARA_FILE_INFOS[charaId].cameraOffsetY),', '(q3_12)Q8_TO_Q12(CHARA_FILE_INFOS[charaId].cameraOffsetY),')
            code = re.sub(r'Gfx_FogOverlayQuadDraw\(([^;]+)\);', lambda m: 'Gfx_FogOverlayQuadDraw('+','.join('(s16)('+x+')' if i<4 else x for i,x in enumerate(m[1].split(',')))+');',code) if name=='func_80045534' else code
            # PORT: Packet pointers keep native width; explicit casts preserve byte offsets.
            pointer_vars = dict((var, typ) for typ, var in re.findall(r'\b(\w+)\*\s+(\w+)\s*;', code))
            for var, typ in pointer_vars.items():
                code = re.sub(r'(?<![*.>\w])(\b'+var+r'\s*=)\s*([^;,\n]+)([;,])', lambda m: m[1]+' ('+typ+'*)('+m[2]+')'+m[3] if '(' not in m[2] else m[0], code)
            prototype = re.sub(r'//[^\n]*', '', code.split('{', 1)[0]).strip() + ';\n'
            header += prototype
            sources.append(code)
    header += '#endif\n'
    for old, new in REBIND.items():
        header = re.sub(r'\b'+old+r'\b', new, header)
        sources = [re.sub(r'\b'+old+r'\b', new, s) for s in sources]
    (out / 'render_generated.h').write_text(header)
    (out / 'render_milestone.rs').write_text(RUST_TESTS)
    constants = initializer(read('src/bodyprog/sys/map_info.c'), 'MAP_EFFECTS_INFOS')
    constants += initializer(read('src/bodyprog/gfx/bodyprog_80055028.c'), 'D_800AE1B4')
    for name in ['D_800A9F80','D_800A9F84','D_800A9F88','D_800A9F8C','D_800A9F98']:
        constants += initializer(read('src/bodyprog/world/world_effects.c'),name)
    constants += 'extern s_800AE204 D_800AE204[26];\n'
    constants += initializer(read('src/bodyprog/gfx/billboard_draw.c'),'D_800AE4DC')
    constants += initializer(read('src/bodyprog/gfx/billboard_draw.c'),'D_800AE500')
    (out / 'render_consumers.c').write_text(NOTICE + '#include "render_generated.h"\n#include "render_services.h"\n#include "render_gte.h"\nstatic s_GteScratchData port_render_scratch;\ns_SysWork_2388 port_render_env;\n' + constants + ''.join(sources))
    # PORT: Bind only rendering calls in generated GPL callers. Player source and
    # its fail-closed guards remain intact and available to the parallel lane.
    path = out / 'gameplay_consumers.c'
    code = path.read_text()
    for old, new in REBIND.items(): code = re.sub(r'\b'+old+r'\b', new, code)
    path.write_text(code if code.startswith('#include "render_generated.h"\n') else '#include "render_generated.h"\n' + code)


def check_clang(compiler):
    """Keep the existing 18-unit gate and add the rendering units at arm64."""
    root=Path(__file__).resolve().parents[1]
    run=subprocess.run(['python',str(root/'tools/check-native-clang.py'),'--compiler',compiler],cwd=root)
    if run.returncode:return run.returncode
    output=root/'target/core-native-clang'
    flags=['-target','aarch64-apple-ios15.0','-ffreestanding','-std=c11','-Wall','-Wextra','-Werror','-nostdinc']
    for directory in [root/'tools/layout-include',root/'port',root/'port/include',output,root/'game/decomp/include',root/'game/decomp/src/main']:
        flags+=['-I',str(directory)]
    for name in ['ccos','csin','csqrt','catan']:flags+=['-fno-builtin-'+name]
    failed=0
    for source in [output/'render_consumers.c',root/'port/render_services.c']:
        run=subprocess.run([compiler,str(source),'--checks=-*,clang-analyzer-core.DivideZero','--warnings-as-errors=*','--',*flags],capture_output=True,text=True)
        print(('PASS' if not run.returncode else 'FAIL')+': '+source.name,flush=True)
        if run.returncode:print(run.stdout+run.stderr,flush=True);failed+=1
    print(f'Native arm64 rendering C: {2-failed} passed, {failed} failed; Apple SDK/runtime not exercised.')
    return int(failed!=0)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--check-clang', metavar='COMPILER')
    args = parser.parse_args()
    if args.check_clang:raise SystemExit(check_clang(args.check_clang))
    if not args.decomp or not args.out:parser.error('--decomp and --out are required for generation')
    generate(args.decomp, args.out)
