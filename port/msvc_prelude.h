/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_MSVC_PRELUDE_H
#define SH_MSVC_PRELUDE_H

// PORT: Keep host size_t; PsyQ's unsigned-int size_t conflicts with MSVC's 64-bit CRT.
#define _SIZE_T
#include <stddef.h>
#include "common.h"

#undef SECTION
// PORT: Use ordinary native storage for GCC section annotations in the compile probe.
// Overlay relocation/execution is still unresolved; this does not make them runnable.
#define SECTION(name)

#endif
