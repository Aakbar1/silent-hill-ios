/* SPDX-License-Identifier: GPL-3.0-only */
#include "boot.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>

_Alignas(8) u8 port_scratch[1024];
_Alignas(8) u8 port_fs_buffers[2][1024*1024];
_Alignas(8) u8 port_packets[2][131072];
_Alignas(8) u8 port_overlay_body[1024*1024], port_overlay_dynamic[1024*1024];
PortGameWork g_GameWork;
PortSysWork g_SysWork;
static PortGameWorkConst config;
PortGameWorkConst* g_GameWorkConst = &config;
static PortController controller;
PortController* g_Controller0 = &controller;
s32 g_ActiveBufferIdx, g_TickCount, g_VBlanks, g_UncappedVBlanks;
s32 g_IntervalVBlanks=1, g_Demo_VideoPresentInterval=1;
q19_12 g_DeltaTime, g_DeltaTimeRaw=TIMESTEP_60_FPS, g_GravitySpeed;
q19_12 g_ScreenFadeTimestep;
s32 g_ScreenFade_Status = ScreenFadeState_None;
GsOT_TAG g_OtTags0[2][16], g_OtTags1[2][ORDERING_TABLE_SIZE];
GsOT g_OrderingTable0[2] = {{11,g_OtTags1[0],0,0,0},{11,g_OtTags1[1],0,0,0}};
GsOT g_OrderingTable2[2] = {{4,g_OtTags0[0],0,0,0},{4,g_OtTags0[1],0,0,0}};
s_FsImageDesc g_Font16AtlasImg={{0,16},0,240,304,511};
s_FsImageDesc g_KonamiLogoImg={{0,12},0,0,0,0};
s_FsImageDesc g_KcetLogoImg={{0,14},0,0,0,1};
DRAWENV GsDRAWENV;
DISPENV GsDISPENV;
PACKET* GsOUT_PACKET_P;

/* Preserve the reference table's 6-bit filename encoding as well as its read metadata. */
#define NAME6(c) (((u32)(u8)(c)-0x20u)&63u)
#define NAME_PART(a,b,c,d) (NAME6(a)|(NAME6(b)<<6)|(NAME6(c)<<12)|(NAME6(d)<<18))
#define FN(a,b,c,d,e,f,g,h) NAME_PART(a,b,c,d),NAME_PART(e,f,g,h)
s_FileInfo g_FileTable[] = {
#include "filetable.c.USA.inc"
};
#undef FN
#undef NAME_PART
#undef NAME6
s32 Fs_GetFileSize(s32 index) { return (s32)g_FileTable[index].blockCount*256; }
s32 Math_MulFixed(s32 a,s32 b,s32 shift) { return (s32)(((s64)a*b)>>shift); }

static jmp_buf stop;
static int stop_code;
static s32 vblanks, active;
static s32 hblanks;
static bool display_enabled;
static s16 buffer_x[2], buffer_y[2];
static void (*vsync_callback)(void);
static const void* token_pointers[16384];
static u32 token_hash[32768];
static u32 token_count;
u32 port_gpu_token(const void* pointer) {
    uintptr_t value=(uintptr_t)pointer;
    if (value<=0xffffff) return (u32)value;
    u32 bucket=(u32)((value>>3)^(value>>16))&32767;
    while (token_hash[bucket]) {
        if (token_pointers[token_hash[bucket]]==pointer) return token_hash[bucket];
        bucket=(bucket+1)&32767;
    }
    if (token_count>=16383) { fprintf(stderr,"OT token registry exhausted\n"); longjmp(stop,2); }
    token_pointers[++token_count]=pointer;
    token_hash[bucket]=token_count;
    return token_count;
}
void* port_gpu_pointer(u32 token) {
    if (!token || token>token_count) { fprintf(stderr,"Invalid OT token %u\n",token); longjmp(stop,2); }
    return (void*)token_pointers[token];
}

void Game_StateStepIncrement(s32 index) {
    g_GameWork.gameStateSteps[index]++;
    for (s32 i=index+1;i<3;i++) g_GameWork.gameStateSteps[i]=0;
    if (!index) g_SysWork.gameStateStepCounter=0;
}
void port_game_state_next(s32 next) {
    printf("STATE %d -> %d at VBlank %d\n",g_GameWork.gameState,next,vblanks);
    g_GameWork.gameStatePrev=g_GameWork.gameState;
    g_GameWork.gameState=next;
    memset(g_GameWork.gameStateSteps,0,sizeof(g_GameWork.gameStateSteps));
    g_SysWork.gameStateCounter=0;
    g_SysWork.gameStateStepCounter=0;
    g_SysWork.sysState=SysState_Gameplay;
    g_SysWork.sysStateCounter=0;
}

// PORT: Asynchronous hardware CD commands are represented by a bounded native job queue.
typedef struct { s32 file; void* destination; s_FsImageDesc* image; } Job;
static Job jobs[32];
static s32 job_first, job_last;
void Fs_QueueInitialize(void) { job_first=job_last=0; }
s32 Fs_QueueGetLength(void) { return job_last-job_first; }
void Fs_QueueStartReadTim(s32 file,void* buffer,s_FsImageDesc* image) {
    if (job_last-job_first>=32) longjmp(stop,2);
    jobs[job_last++%32]=(Job){file,buffer,image};
}
void Fs_QueueStartRead(s32 file,void* buffer) { Fs_QueueStartReadTim(file,buffer,NULL); }
static u32 word(const u8* p) { u32 result; memcpy(&result,p,4); return result; }
void Fs_QueueUpdate(void) {
    if (!Fs_QueueGetLength()) return;
    Job job=jobs[job_first++%32];
    u32 bytes=(u32)Fs_GetFileSize(job.file);
    u32 lba=g_FileTable[job.file].startSector;
    if (bytes>sizeof(port_fs_buffers[0]) || port_read_file((u32)job.file,bytes,(u8*)job.destination)) longjmp(stop,2);
    printf("READ file=%d LBA=%u bytes=%u%s\n",job.file,lba,bytes,job.image?" TIM":"");
    if (job.image) {
        const u8* p=(const u8*)job.destination;
        const u8* end=p+bytes;
        if (word(p)!=0x10) { fprintf(stderr,"Invalid TIM magic\n"); longjmp(stop,2); }
        u32 flags=word(p+4); p+=8;
        if (flags&8) {
            u32 length=word(p);
            if (length<12 || p+length>end) longjmp(stop,2);
            RECT rect; memcpy(&rect,p+4,8);
            rect.x=job.image->clutX; rect.y=job.image->clutY;
            if ((u32)rect.w*(u32)rect.h*2>length-12) longjmp(stop,2);
            port_load_vram(rect.x,rect.y,rect.w,rect.h,(const u16*)(p+12));
            p+=length;
        }
        if (p+12>end) longjmp(stop,2);
        u32 length=word(p);
        RECT rect; memcpy(&rect,p+4,8);
        if (length<12 || p+length>end || (u32)rect.w*(u32)rect.h*2>length-12) longjmp(stop,2);
        rect.x=(s16)(job.image->u+((job.image->tPage[1]&15)<<6));
        rect.y=(s16)(job.image->v+((job.image->tPage[1]<<4)&256));
        port_load_vram(rect.x,rect.y,rect.w,rect.h,(const u16*)(p+12));
    }
}
void Fs_QueueWaitForEmpty(void) { while (Fs_QueueGetLength()) { Fs_QueueUpdate(); VSync(0); } }
// PORT: Decrypted overlays are data only. Their C functions are statically linked native code.
void Fs_DecryptOverlay(s32* dst,const s32* src,s32 bytes) {
    u32 seed=0;
    for (s32 i=0;i<bytes/4;i++) { seed=(seed+0x01309125u)*0x03A452F7u; ((u32*)dst)[i]=((const u32*)src)[i]^seed; }
    printf("OVERLAY decrypted %d bytes; native C entry points linked\n",bytes);
}

int ResetGraph(int mode) {
    (void)mode; active=0;
    port_clear_vram(0,0,1024,512,0,0,0);
    port_draw_env(0,0,1024,512,0,0);
    return 0;
}
void ResetCallback(void) { vsync_callback=NULL; }
int DrawSync(int mode) { (void)mode; return 0; }
int ClearImage2(RECT* rect,u8 r,u8 g,u8 b) { port_clear_vram(rect->x,rect->y,rect->w,rect->h,r,g,b); return 0; }
int LoadImage(RECT* rect,u_long* data) {
    if (!rect || !data || rect->w<=0 || rect->h<=0 || rect->w>1024 || rect->h>512) return -1;
    port_load_vram(rect->x,rect->y,rect->w,rect->h,(const u16*)data); return 0;
}
int StoreImage(RECT* rect,u_long* data) {
    if (!rect || !data) return -1;
    return port_store_vram(rect->x,rect->y,rect->w,rect->h,(u16*)data)?-1:0;
}
int MoveImage(RECT* rect,int x,int y) {
    if (!rect) return -1;
    return port_move_vram(rect->x,rect->y,rect->w,rect->h,x,y)?-1:0;
}
DISPENV* PutDispEnv(DISPENV* env) { GsDISPENV=*env; return env; }
DRAWENV* PutDrawEnv(DRAWENV* env) {
    GsDRAWENV=*env;
    // PORT: Gs exposes a 224-line display window in interlaced mode; render both fields natively.
    s32 height=g_GameWork.gsScreenHeight==448?448:env->clip.h;
    port_draw_env(env->clip.x,env->clip.y,env->clip.w,height,env->ofs[0],env->ofs[1]);
    if (env->isbg) port_clear_vram(env->clip.x,env->clip.y,env->clip.w,height,env->r0,env->g0,env->b0);
    return env;
}
void SetDispMask(int mask) { display_enabled=mask!=0; }
void DrawPrim(void* packet) { port_draw_packet((const u32*)packet,(u32)getlen(packet)+1); }
void AddPrim(void* ot,void* packet) { setaddr(packet,getaddr(ot)); setaddr(ot,packet); }
void SetDrawMode(DR_MODE* p,int dfe,int dtd,int tpage,RECT* tw) {
    (void)tw; p->tag=0x02000000; p->code[0]=0xe1000000u|(u32)tpage|((u32)dtd<<9)|((u32)dfe<<10); p->code[1]=0xe2000000;
}
void GsClearOt(u_short offset,u_short point,GsOT* ot) {
    (void)offset; (void)point;
    u32 count=1u<<ot->length;
    for (u32 i=0;i<count;i++) { ((u32*)ot->org)[i]=i?port_gpu_token(&ot->org[i-1]):0xffffff; }
}
void GsDrawOt(GsOT* ot) {
    port_begin_ot();
    const u32* packet=(const u32*)&ot->org[(1u<<ot->length)-1];
    for (u32 count=0;count<100000;count++) {
        u32 tag=*packet;
        if (tag>>24) port_draw_packet(packet,(tag>>24)+1);
        u32 next=tag&0xffffff;
        if (next==0xffffff) { port_end_ot(); return; }
        packet=(const u32*)port_gpu_pointer(next);
    }
    fprintf(stderr,"OT cycle\n"); longjmp(stop,2);
}
void GsSortClear(u8 r,u8 g,u8 b,GsOT* ot) {
    // PORT: A clear command is injected at the OT tail; it retains normal ordering.
    static TILE clear[2];
    TILE* p=&clear[active]; p->tag=0x03000000; setRGB0(p,r,g,b); p->code=0x60;
    p->x0=(s16)(-g_GameWork.gsScreenWidth/2); p->y0=(s16)(-g_GameWork.gsScreenHeight/2);
    p->w=(s16)g_GameWork.gsScreenWidth; p->h=(s16)g_GameWork.gsScreenHeight;
    AddPrim(&ot->org[(1u<<ot->length)-1],p);
}
int GsGetActiveBuff(void) { return active; }
// PORT: Native VBlank ticks supply deterministic NTSC HBlank counts (263 per tick).
void GsInitVcount(void) { hblanks=0; }
s32 GsGetVcount(void) { return hblanks; }
void GsClearVcount(void) { hblanks=0; }
void GsInitGraph2(u_short x,u_short y,u_short mode,u_short dith,u_short vram) {
    (void)mode; (void)dith; (void)vram;
    GsDISPENV.disp=(RECT){0,32,(s16)x,(s16)y};
    GsDRAWENV.clip=GsDISPENV.disp;
    GsDRAWENV.ofs[0]=(s16)(x/2); GsDRAWENV.ofs[1]=(s16)(32+y/2);
    PutDrawEnv(&GsDRAWENV);
}
void GsDefDispBuff2(u_short x0,u_short y0,u_short x1,u_short y1) {
    buffer_x[0]=(s16)x0; buffer_x[1]=(s16)x1;
    buffer_y[0]=(s16)y0; buffer_y[1]=(s16)y1;
}
void GsSwapDispBuff(void) {
    active^=1;
    GsDISPENV.disp.x=buffer_x[active^1]; GsDISPENV.disp.y=buffer_y[active^1];
    GsDRAWENV.clip.x=buffer_x[active]; GsDRAWENV.clip.y=buffer_y[active];
    GsDRAWENV.ofs[0]=(s16)(buffer_x[active]+g_GameWork.gsScreenWidth/2);
    GsDRAWENV.ofs[1]=(s16)(buffer_y[active]+g_GameWork.gsScreenHeight/2);
    PutDrawEnv(&GsDRAWENV);
}
int VSyncCallback(void (*callback)(void)) { vsync_callback=callback; return 0; }
int VSync(int mode) {
    if (mode<0) return vblanks;
    if (vsync_callback) vsync_callback();
    vblanks++;
    hblanks+=263;
    if (port_present(GsDISPENV.disp.x,GsDISPENV.disp.y,display_enabled?GsDISPENV.disp.w:0,display_enabled?GsDISPENV.disp.h:0,g_GameWork.gameState,g_GameWork.gameStateSteps[0])) longjmp(stop,1);
    return vblanks;
}

// PORT: Stub systems unrelated to logo rendering; every stub logs its first call once.
#define STUB0(name) void name(void) { static bool seen; if (!seen) {printf("STUB " #name "\n");seen=true;} }
#define STUB1(name) void name(s32 value) { (void)value; static bool seen; if (!seen) {printf("STUB " #name "\n");seen=true;} }
STUB0(GsInit3D)
STUB0(Demo_ControllerDataUpdate) STUB0(Demo_Update)
STUB0(Demo_GameRandSeedSet) STUB0(Demo_PresentIntervalUpdate) STUB0(Game_WarmBoot)
STUB0(MemCard_SysInit) STUB0(MemCard_SysEnable) STUB0(MemCard_InitStatus) STUB0(MemCard_Update)
STUB0(ItemScreen_TmdGsFCallInit) STUB0(func_800890B8) STUB0(SD_Init) STUB1(SD_Call)
STUB0(Sd_TaskPoolExecute) STUB1(func_80089090) STUB0(func_80089128) STUB0(func_8008D78C)
STUB0(WorldGfx_HarryCharaLoad) STUB0(GameFs_BgItemLoad) STUB1(Map_EffectTexturesLoad)
STUB0(nullsub_800334C8)
#define STUB_RETURN(name,type,value) type name(void) {static bool seen;if (!seen) {printf("STUB " #name "\n");seen=true;}return value;}
STUB_RETURN(CdInit,int,1) STUB_RETURN(MainLoop_ShouldWarmReset,s32,0)
STUB_RETURN(MemCard_ElementsUpdate,bool,true) STUB_RETURN(Sd_AudioStreamingCheck,s32,0)
STUB0(InitGeom)
void SpuInit(void) { port_spu_reset(); printf("SPU backend reset (silent fallback)\n"); }
// PORT: libpad reads host PadSource packets. Boot uses held bits only; the full
// game-owned joy/libkpad mode, pulse and vibration algorithms remain to be linked.
static u8 pad0[8], pad1[8];
static u8* pad_buffers[2]={pad0,pad1};
static bool pad_started;
void PadInitDirect(u8* first,u8* second) { pad_buffers[0]=first; pad_buffers[1]=second; }
void PadStartCom(void) { pad_started=true; }
void PadStopCom(void) { pad_started=false; }
static void pad_refresh(void) {
    if (!pad_started) return;
    if (pad_buffers[0]) port_pad_read(pad_buffers[0]);
    if (pad_buffers[1]) { memset(pad_buffers[1],0xff,8); }
}
int PadGetState(int port) { pad_refresh(); return port==0 && pad_started?6:0; }
int PadInfoMode(int port,int info,int index) { (void)index; return port==0 && (info==1 || info==2)?7:0; }
int PadSetMainMode(int port,int mode,int lock) { (void)mode;(void)lock;return port==0; }
int PadSetActAlign(int port,u8* alignment) { (void)alignment;return port==0; }
int PadInfoAct(int port,int actuator,int info) { (void)port;(void)actuator;(void)info;return 0; }
void PadSetAct(int port,u8* values,int count) { (void)port;(void)values;(void)count; }
void Joy_Init(void) { PadInitDirect(pad0,pad1); PadStartCom(); }
void Joy_ReadP1(void) { pad_refresh(); }
void Joy_ControllerDataUpdate(void) {
    const u8* p=pad_buffers[0];
    controller.buttonFlags.held=p && p[0]==0 ? (u32)(u16)~((u16)p[2]|((u16)p[3]<<8)):0;
}
void Joy_Update(void) { Joy_ReadP1(); Joy_ControllerDataUpdate(); }
#define STOP_STATE(name) void name(void) {printf("STUB " #name " (beyond boot scope)\n");stop_code=3;longjmp(stop,1);}
PORT_OTHER_STATES(STOP_STATE)

extern int sh_main(void);
// PORT: Terminate the otherwise infinite C boot loop at a host frame/cancel boundary.
// Rust callbacks return before any jump, so only C stack frames are unwound here.
int port_run_game(void) {
    int result=setjmp(stop);
    if (!result) { printf("ENTER native decomp main\n"); return sh_main(); }
    return result==1?stop_code:2;
}
