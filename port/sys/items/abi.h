/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_ITEMS_ABI_H
#define SH_ITEMS_ABI_H
#include <stdint.h>
#include <stddef.h>
typedef struct { uint8_t id, count, command, order; } ShItemsInventoryWire;
typedef struct { uint8_t bytes[636]; } ShItemsSaveWire;
typedef struct { uint8_t bytes[12]; } ShItemsMetadataWire;
typedef struct { uint8_t bytes[16]; } ShItemsSaveElementWire;
typedef struct { uint8_t bytes[28]; } ShItemsCreditStateWire;
typedef struct { uint8_t bytes[88]; } ShItemsCredit3dWire;
typedef struct { uint8_t bytes[12]; } ShItemsMapMarkerWire;
typedef struct { uint8_t bytes[12]; } ShItemsTmdHeaderWire;
typedef struct { uint8_t bytes[28]; } ShItemsTmdObjectWire;
typedef struct { uint8_t x, y; } ShItemsPanelCellWire;
typedef struct { int16_t x, y; } ShItemsPointNative;
typedef struct { int8_t thresholds[11], sequence[5]; } ShItemsPianoNative;
_Static_assert(sizeof(ShItemsInventoryWire)==4,"inventory wire");
_Static_assert(sizeof(ShItemsSaveWire)==636,"save wire");
_Static_assert(sizeof(ShItemsMetadataWire)==12,"metadata wire");
_Static_assert(sizeof(ShItemsSaveElementWire)==16,"save element wire");
_Static_assert(sizeof(ShItemsCreditStateWire)==28,"credits wire");
_Static_assert(sizeof(ShItemsCredit3dWire)==88,"3d credits wire");
_Static_assert(sizeof(ShItemsMapMarkerWire)==12,"map marker wire");
_Static_assert(sizeof(ShItemsTmdHeaderWire)==12,"TMD header wire");
_Static_assert(sizeof(ShItemsTmdObjectWire)==28,"TMD object wire");
_Static_assert(sizeof(ShItemsPanelCellWire)==2,"panel cell wire");
_Static_assert(sizeof(ShItemsPointNative)==4,"point native");
_Static_assert(sizeof(ShItemsPianoNative)==16,"piano native");
int sh_items_piano_decode(const uint8_t* bytes,size_t length,ShItemsPianoNative* out);
int sh_items_point_decode(const uint8_t* bytes,size_t length,ShItemsPointNative* out);
int sh_items_save_layout_probe(const uint8_t* bytes,size_t length);
/* Core's existing callback convention: 0=success, 1=missing, 2=invalid/I/O. */
int port_save_read(uint32_t slot,uint8_t* out,uint32_t length);
int port_save_write(uint32_t slot,const uint8_t* bytes,uint32_t length);
/* Sidecars preserve original metadata/options without extending save payloads.
 * kind 0: 12-byte slot metadata, id 0..329. kind 1: 56-byte options,
 * id=card*15+file, 0..29. Return codes match port_save_read/write. */
int port_items_sidecar_read(uint32_t kind,uint32_t id,uint8_t* out,uint32_t length);
int port_items_sidecar_write(uint32_t kind,uint32_t id,const uint8_t* bytes,uint32_t length);
int sh_items_slot_id(int device,int file,int save,uint32_t* out);
void sh_items_save_service_tick(void);
void sh_items_save_service_reset(void);
#endif
