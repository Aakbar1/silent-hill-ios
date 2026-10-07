/* SPDX-License-Identifier: GPL-3.0-only */
#include "camera.h"
VC_WORK vcWork;
MATRIX GsIDMATRIX={{{4096,0,0},{0,4096,0},{0,0,4096}},{0,0,0}};
MATRIX GsIDMATRIX2={{{4096,0,0},{0,4096,0},{0,0,4096}},{0,0,0}};
MATRIX GsWSMATRIX;
void GsSetLsMatrix(MATRIX* matrix) {SetRotMatrix(matrix);SetTransMatrix(matrix);}
MATRIX* TransposeMatrix(MATRIX* input,MATRIX* output) {
    MATRIX copy=*input;
    for(s32 row=0;row<3;row++)for(s32 column=0;column<3;column++)output->m[row][column]=copy.m[column][row];
    return output;
}
// PORT: Preserve the SDK's signed 15-bit high/low decomposition and both COP2
// stages. A single floating or 64-bit multiply would change its rounding.
VECTOR* ApplyRotMatrixLV(VECTOR* input,VECTOR* output) {
    s32 low[3];u32 high_result[3];
    s32 values[3]={input->vx,input->vy,input->vz};
    for(u32 i=0;i<3;i++) {
        u32 magnitude=values[i]<0?0u-(u32)values[i]:(u32)values[i];
        s32 high=(s32)magnitude>>15;low[i]=(s32)(magnitude&0x7fff);
        if(values[i]<0){high=-high;low[i]=-low[i];}
        port_gte_write_data(9+i,(u32)high);
    }
    (void)port_gte_execute(0x41e012);
    for(u32 i=0;i<3;i++)high_result[i]=port_gte_read_data(25+i)<<3;
    for(u32 i=0;i<3;i++)port_gte_write_data(9+i,(u32)low[i]);
    (void)port_gte_execute(0x49e012);
    output->vx=(s32)(high_result[0]+port_gte_read_data(25));
    output->vy=(s32)(high_result[1]+port_gte_read_data(26));
    output->vz=(s32)(high_result[2]+port_gte_read_data(27));
    return output;
}
VECTOR* ApplyMatrixLV(MATRIX* matrix,VECTOR* input,VECTOR* output) {
    SetRotMatrix(matrix);return ApplyRotMatrixLV(input,output);
}
s32 Map_TypeGet(void) {return (s32)(g_MapOverlayHdr.mapInfo-MAP_INFOS);}
void port_camera_loading_probe(void) {
    VECTOR3 watch=g_SysWork.playerWork.player.position,cam=watch;
    watch.vy=Q12(-0.6f);cam.vy=Q12(-1.0f);cam.vz+=Q12(2.0f);
    vcInitCamera((s_MapOverlayHdr*)&g_MapOverlayHdr,&g_SysWork.playerWork.player.position);
    for(s32 step=0;step<2;step++) {
        vcUserWatchTarget(&watch,NULL,true);vcUserCamTarget(&cam,NULL,true);
        vcMoveAndSetCamera(true,false,false,false,false,false,false,false);
        // PORT: Original view matrices store Q23.8 translations, while camera
        // targets remain Q19.12. Assert the original quantization explicitly.
        if(vcWork.cam_pos.vx!=cam.vx || vcWork.cam_pos.vz!=cam.vz ||
           vwViewPointInfo.worldpos.vx!=(cam.vx>>4)*16 || vwViewPointInfo.worldpos.vz!=(cam.vz>>4)*16)
            port_unimplemented("original loading camera target/view mismatch");
        watch.vx+=Q12(1.0f);cam.vx+=Q12(1.0f);
    }
}
