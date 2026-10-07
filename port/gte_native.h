/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_GTE_NATIVE_H
#define SH_GTE_NATIVE_H
#include "common.h"
#include <string.h>
// PORT: COP2 has per-worker native state owned by psxgpu, never CPU emulation.
void port_gte_reset(void);
u32 port_gte_read_data(u32 reg);
u32 port_gte_read_control(u32 reg);
void port_gte_write_data(u32 reg,u32 value);
void port_gte_write_control(u32 reg,u32 value);
int port_gte_execute(u32 opcode);
void port_gte_store_sxy(void* destination,u32 reg);
static inline u32 port_gte_load32(const void* p) {u32 v;memcpy(&v,p,4);return v;}
static inline u32 port_gte_load16(const void* p) {u16 v;memcpy(&v,p,2);return v;}
static inline void port_gte_store32(void* p,u32 v) {memcpy(p,&v,4);}
static inline void port_gte_store16(void* p,u32 v) {u16 s=(u16)v;memcpy(p,&s,2);}
// PORT: Arithmetic right shift without implementation-defined signed shifts.
static inline u32 port_gte_asr(u32 v,u32 n) {n&=31;return n?((v>>n)|((v&0x80000000u)?(~0u<<(32-n)):0)):v;}
#include <psyq/inline_c.h>
#include <psyq/gtemac.h>
#endif
