/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_RENDER_SERVICES_H
#define SH_RENDER_SERVICES_H
#include "render_generated.h"
void func_80041074(GsOT*,q19_12,const SVECTOR*,const VECTOR3*);
void func_8008D470(q3_12,const SVECTOR*,const VECTOR3*,s_WaterZone*);
void func_8003E740(void);
void Gfx_BillboardDraw(s32,q19_12,q19_12,q19_12,GsOT*,s32);
void port_render_light_tint(u8,u8,u8,u8,u8,u8);
void port_render_average_col(const u8*,const u8*,s32,s32,u8*);
void port_render_model_check(const s_ModelHeader*);
extern u32 port_render_world_models;
void port_render_project(s_MeshHeader*,s32,s_GteScratchData*,MATRIX*);
void port_render_project_world(s_MeshHeader*,s32,s_GteScratchData*,MATRIX*);
u8 port_render_normals(s_MeshHeader*,s32,s_GteScratchData2*);
// PORT: Original signed depth reads keep their bit patterns in native slots.
#define port_render_depth(p,i) ((s16)(p)->screenZ_168[(i)])
#define port_render_normal_color(p,i) ((p)->field_21C[(i)])
#endif
