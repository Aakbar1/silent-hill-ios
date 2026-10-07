/* SPDX-License-Identifier: GPL-3.0-only */
/* Prerequisite check only: no game code, game data, or PsyQ implementation. */
int native_toolchain_pointer_bits(void)
{
    return (int)(sizeof(void*) * 8);
}
