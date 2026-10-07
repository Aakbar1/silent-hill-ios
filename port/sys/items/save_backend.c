/* SPDX-License-Identifier: GPL-3.0-only */
#include "abi.h"
#include "game.h"
#include "bodyprog/memcard.h"
/* PORT: Native slots replace PS1 card filesystem I/O. The original SAVELOAD
 * state paths remain compiled unchanged. Each operation completes on a tick;
 * metadata and options never enlarge the host's exact 636-byte slot payload.
 * Settings/metadata sidecars use explicit core callbacks (see recipe). */
static s_MemCard_SaveMetadata metadata[330];
static s_SaveScreenElement elements[2][180];
static s32 pending, pendingDevice, pendingFile, pendingSave, result;
static s32 selectedFile, selectedSave;
static s32 enabled,dirty=1;
static s32 fileStatus[2][15];
s_SaveScreenElement* sh_items_save_elements(uint32_t slot) { return slot<2 ? elements[slot] : NULL; }
void sh_items_save_service_reset(void) {
    memset(metadata,0,sizeof(metadata)); memset(elements,0,sizeof(elements));memset(fileStatus,0,sizeof(fileStatus));
    pending=0; result=MemCardWorkResult_Success; selectedFile=0; selectedSave=0;
    enabled=0;dirty=1;
}
void MemCard_SysEnable(void) { if(!enabled)dirty=1;enabled=1; }
void MemCard_SysDisable(void) { enabled=0; }
void MemCard_InitStatus(void) {
    uint8_t bytes[636];
    if(!dirty)return;
    dirty=0;memset(fileStatus,0,sizeof(fileStatus));
    for(uint32_t slot=0;slot<330;slot++) {
        int status=port_save_read(slot,bytes,636);
        s_MemCard_SaveMetadata* entry=&metadata[slot];
        int card=(int)(slot/165),file=(int)(slot%165/11);
        if(status==1) { memset(entry,0,sizeof(*entry));continue; }
        if(status || sh_items_save_layout_probe(bytes,636)) { fileStatus[card][file]=FileState_Damaged;continue; }
        status=port_items_sidecar_read(0,slot,(uint8_t*)entry,12);
        if(status==1) {
            s_Savegame data;memcpy(&data,bytes,sizeof(data));
            /* PORT: older native payload-only slots lack PS1 header metadata.
             * Recover display information; retain their 636 bytes untouched. */
            entry->totalSavegameCount=data.savegameCount>0 ? data.savegameCount : 1;
            entry->gameplayTimer=data.gameplayTimer;entry->savegameCount=(u16)data.savegameCount;entry->locationId=(u8)data.locationId;
            entry->isNextFearMode=data.isNextFearMode;entry->add290Hours=data.add290Hours;entry->pickedUpSpecialItemCount=data.pickedUpSpecialItemCount;
        } else if(status || entry->totalSavegameCount<=0) { fileStatus[card][file]=FileState_Damaged;continue; }
        if(fileStatus[card][file]!=FileState_Damaged)fileStatus[card][file]=FileState_Used;
    }
}
s32 MemCard_AllMemCardsStatusGet(void) { return MemCard_StatusStore(MemCardState_Available,0)|MemCard_StatusStore(MemCardState_Available,4); }
s32 MemCard_FileStatusesGet(s32 device) {
    s32 flags=0;if(device!=0 && device!=4)return 0;
    for(int i=0;i<15;i++)flags|=MemCard_FileStatusStore(fileStatus[device/4][i],i);
    return flags;
}
s32 MemCard_UsedFileCount(s32 device) {
    s32 count=0;if(device!=0 && device!=4)return 0;
    for(int i=0;i<15;i++)if(fileStatus[device/4][i]!=FileState_Unused)count++;
    return count;
}
s32 MemCard_FreeFilesCount(s32 device) { return (device==0 || device==4) ? 15-MemCard_UsedFileCount(device) : 0; }
s_MemCard_SaveMetadata* MemCard_SaveMetadataGet(s32 device,s32 file,s32 save) {
    uint32_t slot;
    if (sh_items_slot_id(device,file,save,&slot)) return NULL;
    MemCard_InitStatus();
    return &metadata[slot];
}
bool MemCard_ProcessSet(s32 process,s32 device,s32 file,s32 save) {
    uint32_t slot;
    if (pending) return false;
    if (sh_items_slot_id(device,file,save,&slot) || process<1 || process>6) { result=MemCardWorkResult_FileIoError; return false; }
    if (process==MemCardGameProcessId_Load_Settings) { selectedFile=file; selectedSave=save; }
    pending=process; pendingDevice=device; pendingFile=file; pendingSave=save; result=MemCardWorkResult_Success; return true;
}
s32 MemCard_LastMemCardResultGet(void) { return result; }
void sh_items_save_service_tick(void) {
    uint32_t slot=0;
    uint8_t bytes[636];
    int status=0;
    s32 operation=pending;
    if (!operation) return;
    pending=0;
    if (operation==MemCardGameProcessId_Load_Game) { pendingFile=selectedFile; pendingSave=selectedSave; }
    if (sh_items_slot_id(pendingDevice,pendingFile,pendingSave,&slot)) { result=MemCardWorkResult_FileIoError; return; }
    switch (operation) {
    case MemCardGameProcessId_Save_Game:
        memcpy(bytes,g_SavegamePtr,sizeof(bytes));
        status=sh_items_save_layout_probe(bytes,sizeof(bytes));
        if (!status) status=port_save_write(slot,bytes,636);
        if (!status) {
            s32 biggest=0;
            for(int i=0;i<330;i++)if(metadata[i].totalSavegameCount>biggest)biggest=metadata[i].totalSavegameCount;
            if(biggest==0x7fffffff) { status=2;break; }
            metadata[slot].totalSavegameCount=biggest+1;
            status=port_items_sidecar_write(0,slot,(const uint8_t*)&metadata[slot],12);dirty=1;
        }
        break;
    case MemCardGameProcessId_Load_Game:
        status=port_save_read(slot,bytes,636);
        if (!status) status=sh_items_save_layout_probe(bytes,sizeof(bytes));
        if (!status) memcpy(g_SavegamePtr,bytes,sizeof(bytes));
        break;
    case MemCardGameProcessId_Save_Settings:
        status=port_items_sidecar_write(1,(uint32_t)((pendingDevice/4)*15+pendingFile),(const uint8_t*)&g_GameWork.config,56);break;
    case MemCardGameProcessId_Load_Settings:
        { s_OptionsConfig config;
          status=port_items_sidecar_read(1,(uint32_t)((pendingDevice/4)*15+pendingFile),(uint8_t*)&config,56);
          if(!status)g_GameWork.config=config;
          /* PORT: legacy payload-only slots keep current native configuration. */
          if(status==1)status=0;
        }
        break;
    case MemCardGameProcessId_Init:
        dirty=1;MemCard_InitStatus();break;
    default:
        /* PORT: Native slots have no destructive card-format operation. */
        status=2; break;
    }
    result=status ? MemCardWorkResult_FileIoError : MemCardWorkResult_FileIoComplete;
}
