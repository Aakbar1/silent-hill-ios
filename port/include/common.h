/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_BOOT_COMMON_H
#define SH_BOOT_COMMON_H
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
// PORT: Native CRT types and a boot-only interface avoid unrelated PS1 runtime layouts.
#define _SIZE_T
typedef int8_t s8;
typedef int16_t s16;
typedef int32_t s32;
typedef int64_t s64;
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef s32 q19_12;
typedef s16 q3_12;
typedef u8 q0_8;
typedef unsigned char u_char;
typedef unsigned short u_short;
typedef unsigned int u_int;
// PORT: PsyQ long words are 32 bits even on LP64 iOS. Native sizes use size_t.
typedef uint32_t u_long;
#define _UCHAR_T
#define _USHORT_T
#define _UINT_T
#define _ULONG_T
#define NO_VALUE (-1)
#define STATIC_ASSERT_SIZEOF(type, size) _Static_assert(sizeof(type) == (size), #type)
#define PORT_VER_USA 1
#define PORT_VER_JAP0 0
#define PORT_VER_JAP1 0
#define PORT_VER_JAP2 0
#define VERSION_IS(v) PORT_VER_##v
#define PORT_REGION_NTSCJ 0
#define PORT_REGION_NTSC 1
#define VERSION_REGION_IS(v) PORT_REGION_##v
// PORT: Native storage replaces PS1 section-placement annotations used for binary matching.
#define SECTION(name)
extern _Alignas(8) u8 port_scratch[1024];
// PORT: Scratch RAM becomes native storage; no fixed virtual address dereferences.
#define PSX_SCRATCH ((void*)port_scratch)
#endif
