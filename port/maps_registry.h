/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_MAPS_REGISTRY_H
#define SH_MAPS_REGISTRY_H
#include "map.h"
typedef struct {
    const char* name;
    const s_MapOverlayHdr* (*descriptor)(void);
    void (*reset)(void);
    int (*reset_probe)(void);
    int (*load_data)(void);
    u32 (*point_count)(void);
    u32 (*event_count)(void);
    u32 (*callback_count)(void);
} PortMapEntry;
extern const PortMapEntry port_maps[43];
int port_maps_reset_probe(void);
void port_maps_ground_reset(void);
int port_maps_ground_reset_probe(void);
u32 port_maps_event_count(void);
void port_maps_transition_validate(const s_EventData* event);
void port_maps_callback_validate(u32 index);
#endif
