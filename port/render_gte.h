/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations.
 * Native translations of straight-line register helpers in decomp include/gpu.h.
 */
#ifndef SH_RENDER_GTE_H
#define SH_RENDER_GTE_H
#define gte_lddqa(x) port_gte_write_control(27,(u32)(x))
#define gte_lddqb_0() port_gte_write_control(28,0)
#define gte_ldtr_0() do {for(u32 sh_i=5;sh_i<=7;sh_i++)port_gte_write_control(sh_i,0);} while(0)
#define gte_ldsv_(x) do {u32 sh_x=(u32)(x);for(u32 sh_i=9;sh_i<=11;sh_i++)port_gte_write_data(sh_i,sh_x);} while(0)
#define gte_stIR1() ((s32)port_gte_read_data(9))
#define gte_stSZ3() ((s32)port_gte_read_data(19))
#define gte_ldir_stbk() do {for(u32 sh_i=0;sh_i<3;sh_i++)port_gte_write_data(9+sh_i,port_gte_read_control(13+sh_i));} while(0)
#define gte_ldmac_stir() do {for(u32 sh_i=0;sh_i<3;sh_i++)port_gte_write_control(13+sh_i,port_gte_read_data(25+sh_i));} while(0)
#define gte_ldvxy0_Zero() port_gte_write_data(0,0)
#define gte_ldvz0() port_gte_write_data(1,0)
#define gte_SetVector0(p) gte_ldv0(p)
#define gte_SetRotMatrix_Row0_1(p) do {for(u32 sh_i=0;sh_i<3;sh_i++)port_gte_write_control(sh_i,port_gte_load32((const u8*)(p)+sh_i*4));} while(0)
#define gte_SetRotMatrix_Row2(p) do {for(u32 sh_i=3;sh_i<5;sh_i++)port_gte_write_control(sh_i,port_gte_load32((const u8*)(p)+sh_i*4));} while(0)
#define gte_SetLightSourceXY(x,y) port_gte_write_control(8,(u16)(x)|((u32)(u16)(y)<<16))
#define gte_SetLightSourceZ(z) do {port_gte_write_control(9,(u16)(z));for(u32 sh_i=10;sh_i<=12;sh_i++)port_gte_write_control(sh_i,0);} while(0)
#define gte_SetLightSVector(p) do {port_gte_write_control(8,port_gte_load32(p));port_gte_write_control(9,port_gte_load16((const u8*)(p)+4));for(u32 sh_i=10;sh_i<=12;sh_i++)port_gte_write_control(sh_i,0);} while(0)
#define gte_LoadVector0_XYZ(x,y,z) do {port_gte_write_data(0,(u16)(x)|((u32)(u16)(y)<<16));port_gte_write_data(1,(u32)(z));} while(0)
#define gte_LoadVector0_1_2_XYZ(xy,z) do {for(u32 sh_i=0;sh_i<3;sh_i++){port_gte_write_data(sh_i*2,port_gte_load32((const u8*)(xy)+sh_i*4));port_gte_write_data(sh_i*2+1,port_gte_load16((const u8*)(z)+sh_i*2));}} while(0)
#define gte_FetchScreen0_1_2_XYZ(xy,z) do {for(u32 sh_i=0;sh_i<3;sh_i++){port_gte_store_sxy((u8*)(xy)+sh_i*4,12+sh_i);port_gte_store16((u8*)(z)+sh_i*2,port_gte_read_data(17+sh_i));}} while(0)
// PORT: Preserve the reference's column-order halfword loads, including its
// unusual register-3 order; substituting SetRotMatrix changes the light basis.
static inline void port_render_rot_custom(const void* matrix) {
    const u8* bytes=matrix;
    const u8 low[4]={0,12,8,4},high[4]={6,2,14,10};
    for(u32 i=0;i<4;i++)port_gte_write_control(i,port_gte_load16(bytes+low[i])|((u32)port_gte_load16(bytes+high[i])<<16));
    port_gte_write_control(4,port_gte_load16(bytes+16));
}
#define gte_SetRotMatrix_custom(p) port_render_rot_custom(p)
#endif
