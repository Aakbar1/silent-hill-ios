/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef PSXGPU_PORTABLE_H
#define PSXGPU_PORTABLE_H
#include <stdint.h>
#include <string.h>
typedef int8_t s8;
typedef uint8_t u8;
typedef int16_t s16;
typedef uint16_t u16;
typedef int32_t s32;
typedef uint32_t u32;
typedef int64_t s64;
typedef uint64_t u64;
typedef union {
    u32 r[32];
    union { u32 d; struct { u16 l,h; } w; struct { s16 l,h; } sw;
        struct { u8 l,h,h2,h3; } b; } p[32];
} GteRegs;
typedef struct psxCP2Regs { GteRegs CP2D,CP2C; } psxCP2Regs;
#define likely(x) (x)
#define unlikely(x) (x)
#define force_inline
#if defined(_MSC_VER)
#define noinline __declspec(noinline)
#define attr_aligned(x) __declspec(align(x))
#else
#define noinline __attribute__((noinline))
#define attr_aligned(x) __attribute__((aligned(x)))
#endif
static inline int add_overflow(s64 a, s64 b, s64 *out) {
    u64 bits=(u64)a+(u64)b;
    memcpy(out,&bits,sizeof(bits));
    return ((a^*out)&(b^*out))<0;
}
static inline int sub_overflow(s64 a, s64 b, s64 *out) {
    u64 bits=(u64)a-(u64)b;
    memcpy(out,&bits,sizeof(bits));
    return ((a^b)&(a^*out))<0;
}
static inline u32 clz32(u32 value) {
    u32 n=0;
    if (!value) return 32;
    while ((value&0x80000000u)==0) { n++; value<<=1; }
    return n;
}
#endif
