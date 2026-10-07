/* SPDX-License-Identifier: GPL-3.0-only */
#include "disk32.h"
#ifdef SH_CHECK_IOS_LAYOUT
_Static_assert(sizeof(void*)==8, "iOS arm64 pointers");
_Static_assert(sizeof(long)==8, "iOS LP64 long");
#endif
#ifdef SH_CHECK_BOOT_LAYOUT
#include "boot.h"
STATIC_ASSERT_SIZEOF(u_long,4);
STATIC_ASSERT_SIZEOF(MATRIX,32);
STATIC_ASSERT_SIZEOF(VECTOR,16);
STATIC_ASSERT_SIZEOF(SVECTOR,8);
// P_TAG contains both the 4-byte link/length tag and the 4-byte RGB/opcode word.
STATIC_ASSERT_SIZEOF(P_TAG,8);
#endif
int sh_disk32_layout_check(void) { return (int)sizeof(ShNativeSpan); }
int sh_native_span_read16(const ShNativeSpan* span,size_t index,size_t field,uint16_t* out) { return sh_span_u16(span,index,field,out); }
int sh_native_span_read32(const ShNativeSpan* span,size_t index,size_t field,uint32_t* out) { return sh_span_u32(span,index,field,out); }
