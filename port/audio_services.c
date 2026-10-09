/* SPDX-License-Identifier: GPL-3.0-only */
#include "audio_records.h"
#include <stdio.h>
extern s32 port_audio_trace_enabled(void);
void audio_trace(const char* event,s32 value) {
    if(port_audio_trace_enabled())printf("AUDIO %s value=%d ctrl=%x main=%x,%x cd=%x,%x\n",event,value,
        port_spu_read(0x1aa),port_spu_read(0x180),port_spu_read(0x182),port_spu_read(0x1b0),port_spu_read(0x1b2));
}
// PORT: Observe real voice-register writes; this diagnostic changes no sound state.
void audio_positional_trace(s32 id,s32 voice,s32 pan,s32 attenuation,s32 pitch) {
    if(port_audio_trace_enabled() && id==1358)
        printf("POSITIONAL_SFX tick=%d id=%d voice=%d pan=%d attenuation=%d pitch=%d left=%u right=%u spu_pitch=%u\n",
            VSync(-1),id,voice,pan,attenuation,pitch,
            port_spu_read((u16)(voice*16)),port_spu_read((u16)(voice*16+2)),port_spu_read((u16)(voice*16+4)));
}
_Alignas(8) u8 audio_cd_buffer[0x10000],audio_vab_headers[4][0x10000],audio_kdt_buffer[0x10000];
s32 g_RadioPitchState;
s_BgmLayerLimits audio_map0_limits;
s16 audio_map0_flags[8];
void audio_map0_tables(void) {
    static bool loaded;
    if(loaded)return;
    // PORT: Native scalar tables from the owned MAP0_S00 overlay. The map's
    // linked base is 0x800C9578; no disk pointer is dereferenced or widened.
    if(port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15d80,8,(u8*)&audio_map0_limits) ||
       port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15d88,16,(u8*)audio_map0_flags))
        port_unimplemented("MAP0_S00 audio scalar tables");
    loaded=true;
}
void audio_ambience_probe(void) {
    // PORT: Explicit audio-only diagnostic after the real gameplay guard;
    // advance original BGM/task/timer services, never fake player progression.
    printf("AUDIO_AMBIENCE_PROBE begin (gameplay remains guarded)\n");
    for(s32 i=0;i<300;i++) {Bgm_Update(false);Sd_TaskPoolExecute();audio_clock_vblank();}
    printf("AUDIO_AMBIENCE_PROBE end midi_task=%u\n",Sd_MidiChannelTaskGet());
    if(!audio_layer_stop_probe())port_unimplemented("BGM stop-sentinel regression");
    printf("AUDIO_LAYER_STOP_PROBE PASS raw_task=65535\n");
}
static bool (*timer_handler)(void);
static bool timer_running,timer_enabled;
static u32 timer_period=7328;
static u64 sample_clock,timer_phase;
static u32 pending_key_on;
extern s32 port_audio_advance(u64 sample);
extern s32 port_audio_read_sectors(u32 lba,u32 count,u8* data);
extern s32 port_audio_cd_command(s32 command,const u8* data);
extern s32 port_audio_reverb(s32 mode,s32 clear);
extern void port_audio_cd_matrix(const u8* matrix);
extern void port_audio_xa_tick(void);
void audio_clock_reset(void) {
    sample_clock=0;timer_phase=0;timer_handler=NULL;
    timer_running=false;timer_enabled=false;pending_key_on=0;
    (void)port_audio_cd_command(CdlPause,NULL);
}

// PORT: Synchronous SDK commands run on the sole game worker. The device
// callback consumes PCM only; timer callbacks never run under a Rust borrow.
s32 audio_timer_open(bool (*callback)(void)) {timer_handler=callback;return 1;}
void audio_timer_period(u32 period) {timer_period=period;timer_phase=0;}
void audio_timer_start(void) {timer_running=true;}
void audio_timer_stop(void) {timer_running=false;}
void audio_timer_enable(bool enabled) {timer_enabled=enabled;}
void EnterCriticalSection(void) {}
void ExitCriticalSection(void) {}
void audio_clock_vblank(void) {
    u64 target=sample_clock+735;
    port_audio_xa_tick();
    while(sample_clock<target) {
        u64 remaining=target-sample_clock;
        u64 until=remaining;
        if(timer_running && timer_enabled && timer_handler) {
            u64 period=(u64)timer_period*8*44100;
            until=(period-timer_phase+33868800-1)/33868800;
            if(until>remaining)until=remaining;
            timer_phase+=until*33868800;
            sample_clock+=until;
            if(port_audio_advance(sample_clock))port_unimplemented("audio mixer advance");
            pending_key_on=0;
            if(timer_phase>=period) {timer_phase-=period;(void)timer_handler();}
        } else {
            sample_clock=target;
            if(port_audio_advance(sample_clock))port_unimplemented("audio mixer advance");
        }
    }
}
static u16 rd(u16 reg) {return port_spu_read(reg);}
static void wr(u16 reg,u16 value) {if(port_spu_write(reg,value))port_unimplemented("SPU register write");}
static u32 keyed;
static u32 transfer_address;
void SpuInit(void) {port_spu_reset();keyed=0;pending_key_on=0;transfer_address=0;}
void SpuQuit(void) {wr(0x1aa,0);keyed=0;}
// PORT: libsd owns its allocation table and fixed bank addresses. PsyQ's
// record-buffer registration is unused after initialization in this driver.
s32 SpuInitMalloc(s32 count,char* memory) {(void)memory;return count;}
void SpuSetKey(s32 on,u32 mask) {
    audio_trace(on?"key_on":"key_off",(s32)mask);
    if(on)keyed|=mask;else keyed&=~mask;
    if(on)pending_key_on|=mask;else pending_key_on&=~mask;
    u16 offset=on?0x188:0x18c;wr(offset,(u16)mask);wr(offset+2,(u16)(mask>>16));
}
s32 SpuGetKeyStatus(u32 mask) {
    if(!mask || (mask&0xffffff)==0)return -1;
    u32 voice=0;while(!(mask&(1u<<voice)))voice++;
    bool env=rd((u16)(voice*16+12))!=0;
    return keyed&(1u<<voice)?(env || (pending_key_on&(1u<<voice))?1:3):(env?2:0);
}
static bool selected(u32 mask,u32 bit) {return mask==0 || (mask&(1u<<bit))!=0;}
static u16 volume(s16 val,s16 mode) {
    static const u16 modes[8]={0,0x8000,0x9000,0xa000,0xb000,0xc000,0xd000,0xe000};
    if(mode<0 || mode>7)port_unimplemented("invalid SPU volume mode");
    return mode?(u16)(modes[mode]|(val&127)):(u16)(val&0x7fff);
}
void SpuSetVoiceAttr(SpuVoiceAttr* a) {
    if(a->mask&0x60)port_unimplemented("SPU note attributes (use Note2Pitch)");
    for(u32 v=0;v<24;v++)if(a->voice&(1u<<v)) {
        u16 b=(u16)(v*16);
        if(selected(a->mask,0))wr(b,volume(a->volume.left,selected(a->mask,2)?a->volmode.left:0));
        if(selected(a->mask,1))wr(b+2,volume(a->volume.right,selected(a->mask,3)?a->volmode.right:0));
        if(selected(a->mask,4))wr(b+4,a->pitch);
        if(selected(a->mask,7))wr(b+6,(u16)((a->addr+7)/8));
        if(selected(a->mask,16))wr(b+14,(u16)((a->loop_addr+7)/8));
        u16 first=selected(a->mask,17)?a->adsr1:rd(b+8);
        u16 second=selected(a->mask,18)?a->adsr2:rd(b+10);
        if(selected(a->mask,8))first=(u16)((first&0x7fff)|(a->a_mode==5?0x8000:0));
        if(selected(a->mask,9))second=(u16)((second&0x3fff)|(a->s_mode==3?0x4000:a->s_mode==5?0x8000:a->s_mode==7?0xc000:0));
        if(selected(a->mask,10))second=(u16)((second&~32)|(a->r_mode==7?32:0));
        if(selected(a->mask,11))first=(u16)((first&~0x7f00)|((a->ar&127)<<8));
        if(selected(a->mask,12))first=(u16)((first&~0xf0)|((a->dr&15)<<4));
        if(selected(a->mask,13))second=(u16)((second&~0x1fc0)|((a->sr&127)<<6));
        if(selected(a->mask,14))second=(u16)((second&~31)|(a->rr&31));
        if(selected(a->mask,15))first=(u16)((first&~15)|(a->sl&15));
        wr(b+8,first);wr(b+10,second);
    }
}
void SpuGetVoiceAttr(SpuVoiceAttr* a) {
    u32 mask=a->voice&0xffffff;
    if(!mask)port_unimplemented("SPU voice get mask");
    u32 v=0;while(!(mask&(1u<<v)))v++;
    u16 b=(u16)(v*16);
    a->volume.left=(s16)((s16)(rd(b)<<1)/2);a->volume.right=(s16)((s16)(rd(b+2)<<1)/2);
    a->pitch=rd(b+4);a->addr=(u32)rd(b+6)*8;a->adsr1=rd(b+8);a->adsr2=rd(b+10);a->envx=(s16)rd(b+12);a->loop_addr=(u32)rd(b+14)*8;
    a->ar=(a->adsr1>>8)&127;a->dr=(a->adsr1>>4)&15;a->sl=a->adsr1&15;
    a->sr=(a->adsr2>>6)&127;a->rr=a->adsr2&31;
    a->a_mode=a->adsr1&0x8000?5:1;a->r_mode=a->adsr2&32?7:3;
    a->s_mode=(a->adsr2>>14)==0?1:(a->adsr2>>14)==1?3:(a->adsr2>>14)==2?5:7;
}
void SpuSetKeyOnWithAttr(SpuVoiceAttr* a) {SpuSetVoiceAttr(a);SpuSetKey(1,a->voice);}
void SpuSetCommonAttr(SpuCommonAttr* a) {
    if(selected(a->mask,0))wr(0x180,volume(a->mvol.left,selected(a->mask,2)?a->mvolmode.left:0));
    if(selected(a->mask,1))wr(0x182,volume(a->mvol.right,selected(a->mask,3)?a->mvolmode.right:0));
    if(selected(a->mask,6))wr(0x1b0,(u16)a->cd.volume.left);
    if(selected(a->mask,7))wr(0x1b2,(u16)a->cd.volume.right);
    if(selected(a->mask,10))wr(0x1b4,(u16)a->ext.volume.left);
    if(selected(a->mask,11))wr(0x1b6,(u16)a->ext.volume.right);
    u16 ctrl=rd(0x1aa);
    for(u32 i=0;i<4;i++) {
        u32 bits[4]={9,13,8,12};
        if(selected(a->mask,bits[i])) {
            s32 val=i==0?a->cd.mix:i==1?a->ext.mix:i==2?a->cd.reverb:a->ext.reverb;
            ctrl=(u16)((ctrl&~(1u<<i))|(val?(1u<<i):0));
        }
    }
    wr(0x1aa,ctrl);
}
s32 SpuSetReverb(s32 on) {u16 c=rd(0x1aa);wr(0x1aa,(u16)((c&~0x80)|(on?0x80:0)));return on;}
s32 SpuSetReverbModeParam(SpuReverbAttr* a) {
    if(a->mask&0x18)return SPU_INVALID_ARGS;
    if(selected(a->mask,0)) {if(port_audio_reverb(a->mode&0xff,0))return SPU_INVALID_ARGS;}
    if(selected(a->mask,1))wr(0x184,(u16)a->depth.left);
    if(selected(a->mask,2))wr(0x186,(u16)a->depth.right);
    return 0;
}
s32 SpuClearReverbWorkArea(s32 mode) {return port_audio_reverb(mode,1)?SPU_INVALID_ARGS:0;}
s32 SpuReserveReverbWorkArea(s32 on) {return on;}
u32 SpuSetReverbVoice(s32 on,u32 mask) {
    u32 old=rd(0x198)|((u32)rd(0x19a)<<16);old=on?(old|mask):(old&~mask);
    wr(0x198,(u16)old);wr(0x19a,(u16)(old>>16));return old;
}
u32 SpuGetReverbVoice(void) {return rd(0x198)|((u32)rd(0x19a)<<16);}
s32 SpuSetTransferMode(s32 mode) {return mode;}
u32 SpuSetTransferStartAddr(u32 addr) {transfer_address=(addr+7)&~7u;return transfer_address;}
u32 SpuWrite(u8* data,u32 size) {
    audio_trace("DMA",(s32)size);
    if(port_spu_transfer(transfer_address,data,size))port_unimplemented("SPU DMA transfer");
    transfer_address=(transfer_address+size)&0x7ffff;return size;
}
s32 SpuIsTransferCompleted(s32 mode) {(void)mode;return 1;}
static u32 cd_lba;
CdlLOC* CdIntToPos(s32 lba,CdlLOC* loc) {
    lba+=150;loc->minute=itob(lba/4500);loc->second=itob((lba/75)%60);loc->sector=itob(lba%75);return loc;
}
static s32 bcd(u8 b) {return (b>>4)*10+(b&15);}
s32 CdSync(s32 mode,u8* data) {(void)mode;if(data)*data=0;return CdlComplete;}
s32 CdReadSync(s32 mode,u8* data) {(void)mode;if(data)*data=0;return 0;}
s32 CdControl(u8 command,u8* data,u8* result) {
    if(result)*result=0;
    if(command==CdlSetloc || command==CdlSeekL) {
        CdlLOC* loc=(CdlLOC*)data;
        if(!loc)return 0;
        cd_lba=(u32)(bcd(loc->minute)*4500+bcd(loc->second)*75+bcd(loc->sector)-150);
    }
    return port_audio_cd_command(command,data)==0;
}
s32 CdControlB(u8 command,u8* data,u8* result) {return CdControl(command,data,result);}
s32 CdRead(s32 sectors,u32* data,s32 mode) {
    (void)mode;
    if(sectors<=0 || sectors>32 || (u8*)data!=audio_cd_buffer)port_unimplemented("audio CD read bounds");
    if(port_audio_read_sectors(cd_lba,(u32)sectors,(u8*)data))port_unimplemented("audio CD read");
    return 1;
}
s32 CdReset(s32 mode) {(void)mode;return port_audio_cd_command(CdlPause,NULL)==0;}
s32 CdMix(CdlATV* matrix) {port_audio_cd_matrix((u8*)matrix);return 1;}
