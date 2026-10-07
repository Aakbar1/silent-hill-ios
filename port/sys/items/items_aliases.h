/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_ITEMS_ALIASES_H
#define SH_ITEMS_ALIASES_H
/* PORT: Preserve original cross-symbol BSS aliasing in explicitly contiguous
 * native allocations. No reliance on linker order or out-of-bounds indexing. */
extern GsF_LIGHT sh_items_lights[20];
extern GsDOBJ2 sh_items_models[10];
extern s32 sh_items_slot_indices[10];
#define g_Items_Lights ((GsF_LIGHT (*)[2])sh_items_lights)
#define D_800C3A88 (sh_items_lights+14)
#define D_800C3AC8 (sh_items_lights+18)
#define g_Items_ItemsModelData sh_items_models
#define D_800C3E08 (sh_items_models[9])
#define D_800C3E18 sh_items_slot_indices
#define g_Inventory_EquippedItemIdx (sh_items_slot_indices[7])
#define __pad_bss_800C3E38 (sh_items_slot_indices+8)
_Static_assert(sizeof(sh_items_lights)==320,"original light alias block");
_Static_assert(sizeof(sh_items_slot_indices)==40,"original index alias block");
#endif
