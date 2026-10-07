/* SPDX-License-Identifier: GPL-3.0-only */
/* PORT: Keep 24-bit GPU tokens, never truncate a native pointer. Core supplies
 * port_gpu_token; synthetic tests do not claim to exercise rasterization. */
#ifndef SH_ITEMS_GPU_H
#define SH_ITEMS_GPU_H
u32 port_gpu_token(const void* pointer);
#undef setaddr
#define setaddr(p,a) (((P_TAG*)(p))->addr=port_gpu_token((const void*)(uintptr_t)(a)))
#undef setRGB0
#undef setRGB1
#undef setRGB2
#undef setRGB3
#define setRGB0(p,r,g,b) ((p)->r0=(u8)(r),(p)->g0=(u8)(g),(p)->b0=(u8)(b))
#define setRGB1(p,r,g,b) ((p)->r1=(u8)(r),(p)->g1=(u8)(g),(p)->b1=(u8)(b))
#define setRGB2(p,r,g,b) ((p)->r2=(u8)(r),(p)->g2=(u8)(g),(p)->b2=(u8)(b))
#define setRGB3(p,r,g,b) ((p)->r3=(u8)(r),(p)->g3=(u8)(g),(p)->b3=(u8)(b))
#undef setXY0
#undef setXY2
#undef setXY3
#undef setXY4
#undef setXYWH
#define setXY0(p,x,y) ((p)->x0=(s16)(x),(p)->y0=(s16)(y))
#define setXY2(p,a,b,c,d) (setXY0(p,a,b),(p)->x1=(s16)(c),(p)->y1=(s16)(d))
#define setXY3(p,a,b,c,d,e,f) (setXY2(p,a,b,c,d),(p)->x2=(s16)(e),(p)->y2=(s16)(f))
#define setXY4(p,a,b,c,d,e,f,g,h) (setXY3(p,a,b,c,d,e,f),(p)->x3=(s16)(g),(p)->y3=(s16)(h))
#define setXYWH(p,x,y,w,h) setXY4(p,x,y,(x)+(w),y,x,(y)+(h),(x)+(w),(y)+(h))
#undef setUV4
#define setUV4(p,a,b,c,d,e,f,g,h) ((p)->u0=(u8)(a),(p)->v0=(u8)(b),(p)->u1=(u8)(c),(p)->v1=(u8)(d),(p)->u2=(u8)(e),(p)->v2=(u8)(f),(p)->u3=(u8)(g),(p)->v3=(u8)(h))
#endif
