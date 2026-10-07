/* SPDX-License-Identifier: GPL-3.0-only */
#include "boot.h"

// PORT: Native PsyQ entry points delegate all commands to psxgpu's exact GTE.
void InitGeom(void) {
    port_gte_reset();
    SetGeomScreen(1000);
    port_gte_write_control(29,341); port_gte_write_control(30,256);
    port_gte_write_control(27,(u32)-98); port_gte_write_control(28,0x01400000);
}
void SetRotMatrix(MATRIX* m) {gte_SetRotMatrix(m);}
void SetLightMatrix(MATRIX* m) {gte_SetLightMatrix(m);}
void SetColorMatrix(MATRIX* m) {gte_SetColorMatrix(m);}
void SetTransMatrix(MATRIX* m) {gte_SetTransMatrix(m);}
void SetGeomOffset(s32 x,s32 y) {gte_SetGeomOffset(x,y);}
void SetGeomScreen(s32 h) {gte_SetGeomScreen(h);}
void SetBackColor(s32 r,s32 g,s32 b) {gte_SetBackColor(r,g,b);}
void SetFarColor(s32 r,s32 g,s32 b) {gte_SetFarColor(r,g,b);}
void ReadRotMatrix(MATRIX* m) {gte_ReadRotMatrix(m);}
void ReadLightMatrix(MATRIX* m) {gte_ReadLightMatrix(m);}
void ReadColorMatrix(MATRIX* m) {gte_ReadColorMatrix(m);}
void ReadGeomOffset(s32* x,s32* y) {gte_ReadGeomOffset(x,y);}
s32 ReadGeomScreen(void) {s32 h;gte_ReadGeomScreen(&h);return h;}
void SetRGBcd(CVECTOR* c) {gte_SetRGBcd(c);}
s32 RotTransPers(SVECTOR* v,s32* xy,s32* p,s32* flag) {s32 z;gte_RotTransPers(v,xy,p,flag,&z);return z;}
s32 RotTransPers3(SVECTOR* a,SVECTOR* b,SVECTOR* c,s32* x,s32* y,s32* z,s32* p,s32* flag) {s32 depth;gte_RotTransPers3(a,b,c,x,y,z,p,flag,&depth);return depth;}
void RotTrans(SVECTOR* v,VECTOR* out,s32* flag) {gte_RotTrans(v,out,flag);}
void RotTransSV(SVECTOR* v,SVECTOR* out,s32* flag) {gte_ldv0(v);gte_rt();gte_stsv(out);gte_stflg(flag);}
VECTOR* ApplyMatrix(MATRIX* m,SVECTOR* v,VECTOR* out) {gte_ApplyMatrix(m,v,out);return out;}
VECTOR* ApplyRotMatrix(SVECTOR* v,VECTOR* out) {gte_ApplyRotMatrix(v,out);return out;}
SVECTOR* ApplyMatrixSV(MATRIX* m,SVECTOR* v,SVECTOR* out) {gte_ApplyMatrixSV(m,v,out);return out;}
MATRIX* MulMatrix0(MATRIX* a,MATRIX* b,MATRIX* out) {MATRIX temp={0};gte_MulMatrix0(a,b,&temp);memcpy(out->m,temp.m,sizeof(temp.m));return out;}
MATRIX* MulMatrix(MATRIX* a,MATRIX* b) {MATRIX out=*a;MulMatrix0(a,b,&out);*a=out;return a;}
MATRIX* MulMatrix2(MATRIX* a,MATRIX* b) {MATRIX out=*b;MulMatrix0(a,b,&out);*b=out;return b;}
void LocalLight(SVECTOR* v,VECTOR* out) {gte_LocalLight(v,out);}
void LightColor(VECTOR* v,VECTOR* out) {gte_LightColor(v,out);}
void DpqColor(CVECTOR* v,s32 p,CVECTOR* out) {gte_DpqColor(v,p,out);}
void NormalColor(SVECTOR* v,CVECTOR* out) {gte_NormalColor(v,out);}
void NormalColorCol(SVECTOR* v,CVECTOR* c,CVECTOR* out) {gte_NormalColorCol(v,c,out);}
void NormalColorDpq(SVECTOR* v,CVECTOR* c,s32 p,CVECTOR* out) {gte_NormalColorDpq(v,c,p,out);}
VECTOR* Square0(VECTOR* v,VECTOR* out) {gte_Square0(v,out);return out;}
VECTOR* Square12(VECTOR* v,VECTOR* out) {gte_Square12(v,out);return out;}
s32 NormalClip(s32 a,s32 b,s32 c) {s32 result;gte_ldsxy3(a,b,c);gte_nclip();gte_stopz(&result);return result;}
s32 AverageZ3(s32 a,s32 b,s32 c) {s32 result;gte_ldsz3(a,b,c);gte_avsz3();gte_stotz(&result);return result;}
s32 AverageZ4(s32 a,s32 b,s32 c,s32 d) {s32 result;gte_ldsz4(a,b,c,d);gte_avsz4();gte_stotz(&result);return result;}

int port_gte_bridge_probe(u32* out) {
    MATRIX m={{{4096,0,0},{0,4096,0},{0,0,4096}},{0,0,0}};
    SVECTOR v={1000,500,2000,0}; VECTOR transformed={0};
    InitGeom();SetRotMatrix(&m);SetTransMatrix(&m);SetGeomScreen(200);SetGeomOffset(0,0);
    out[2]=(u32)RotTransPers(&v,(s32*)&out[0],(s32*)&out[1],(s32*)&out[3]);
    ApplyMatrix(&m,&v,&transformed);
    out[4]=(u32)transformed.vx;out[5]=(u32)transformed.vy;out[6]=(u32)transformed.vz;
    out[7]=(u32)port_gte_execute(0);
    return 1;
}
