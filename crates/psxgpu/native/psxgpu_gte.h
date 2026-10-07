/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef PSXGPU_GTE_H
#define PSXGPU_GTE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Little-endian x64/arm64. Explicit 32-bit registers, no global CPU state. */
typedef struct { uint32_t data[32]; uint32_t control[32]; } psxgpu_gte;
uint32_t psxgpu_gte_read_data(psxgpu_gte *state, uint32_t reg);
void psxgpu_gte_write_data(psxgpu_gte *state, uint32_t reg, uint32_t value);
uint32_t psxgpu_gte_read_control(const psxgpu_gte *state, uint32_t reg);
void psxgpu_gte_write_control(psxgpu_gte *state, uint32_t reg, uint32_t value);
/* Returns 0 on success, -1 for an undefined command (state unchanged). */
int psxgpu_gte_execute(psxgpu_gte *state, uint32_t opcode);
#ifdef __cplusplus
}
#endif
#endif
