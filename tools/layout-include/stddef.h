/* SPDX-License-Identifier: GPL-3.0-only; compile-only target builtin declarations. */
#ifndef SH_LAYOUT_STDDEF_H
#define SH_LAYOUT_STDDEF_H
typedef __SIZE_TYPE__ size_t;
typedef __PTRDIFF_TYPE__ ptrdiff_t;
#define offsetof(T,F) __builtin_offsetof(T,F)
#define NULL ((void*)0)
#endif
