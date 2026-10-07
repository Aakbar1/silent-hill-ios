/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_BOOT_H
#define SH_BOOT_H
#include "common.h"
#include <psyq/libgte.h>
#include <psyq/libgpu.h>
// PORT: MSVC allocates mixed-type PsyQ GsOT_TAG bitfields as 8 bytes. Keep the wire tag 4 bytes.
typedef u8 PACKET;
typedef u32 GsOT_TAG;
typedef struct { u32 length; GsOT_TAG* org; u32 offset, point; GsOT_TAG* tag; } GsOT;
STATIC_ASSERT_SIZEOF(GsOT_TAG,4);
extern DRAWENV GsDRAWENV;
extern DISPENV GsDISPENV;
extern PACKET* GsOUT_PACKET_P;
void GsClearOt(u_short offset,u_short point,GsOT* ot);
void GsDrawOt(GsOT* ot);
void GsSortClear(u8 r,u8 g,u8 b,GsOT* ot);
int GsGetActiveBuff(void);
void GsInitVcount(void);
s32 GsGetVcount(void);
void GsClearVcount(void);
void GsInitGraph2(u_short x,u_short y,u_short mode,u_short dtd,u_short vram);
void GsDefDispBuff2(u_short x0,u_short y0,u_short x1,u_short y1);
void GsInit3D(void);
void GsSwapDispBuff(void);
enum { PRIM_RECT=0x60, RECT_TEXTURE=4, RECT_BLEND=2 };
#define getTPageN(tp,abr,xn,yn) (((tp)<<7)|((abr)<<5)|((yn)<<4)|(xn))
#define setRECTFast(p,_x,_y,_w,_h) ((p)->x=(s16)(_x),(p)->y=(s16)(_y),(p)->w=(s16)(_w),(p)->h=(s16)(_h))
#define setCodeWord(p,c,rgb) (*(u32*)((u8*)(p)+4)=((u32)(c)<<24)|((u32)(rgb)&0xffffff))
#define setWHFast(p,_w,_h) (*(u32*)&(p)->w=(u32)(u16)(_w)|((u32)(u16)(_h)<<16))
#define setUV0AndClut(p,u,v,cx,cy) (*(u32*)&(p)->u0=(u32)(u)|((u32)(v)<<8)|((u32)(((cy)<<6)|((cx)>>4))<<16))
#define setRGBC0(p,r,g,b,c) (*(u32*)&(p)->r0=(u32)(r)|((u32)(g)<<8)|((u32)(b)<<16)|((u32)(c)<<24))
#define addPrimFast(ot,p,len) ((p)->tag=getaddr(ot)|((u32)(len)<<24),setaddr(ot,p))
#include "bodyprog/math/arithmetic.h"
#include "bodyprog/math/constants.h"
#include "bodyprog/math/fixed_point.h"
#include "main/fileinfo.h"

// PORT: Boot runtime records are native objects, never views of game-file bytes.
// Full gameplay records must be migrated separately; upstream assertions stay intact in the baseline gate.
typedef struct { u8 tPage[2], u, v; s16 clutX, clutY; } s_FsImageDesc;
STATIC_ASSERT_SIZEOF(s_FsImageDesc, 8);
STATIC_ASSERT_SIZEOF(SPRT, 20);
STATIC_ASSERT_SIZEOF(TILE, 16);
STATIC_ASSERT_SIZEOF(DR_TPAGE, 8);
typedef struct {
    s32 gameState, gameStatePrev, gameStateSteps[3];
    struct { u8 r, g, b; } background2dColor;
    s32 gsScreenWidth, gsScreenHeight;
} PortGameWork;
typedef struct {
    s32 gameStateCounter, gameStateStepCounter, sysStateCounter, sysState;
    s32 sysFlags, bgmStatusFlags;
} PortSysWork;
typedef struct { struct { s32 screenPositionX, screenPositionY; } config; } PortGameWorkConst;
typedef struct { struct { u32 held; } buttonFlags; } PortController;
extern PortGameWork g_GameWork;
extern PortSysWork g_SysWork;
extern PortGameWorkConst* g_GameWorkConst;
extern PortController* g_Controller0;

enum { GameState_Init=0, GameState_KonamiLogo=1, GameState_KcetLogo=2,
 GameState_MainLoadScreen=10, GameState_InGame=11, GameState_InventoryScreen=14,
 SysState_Gameplay=0, SysFlag_DemoActive=2, BgmStatusFlag_None=0, BgmStatusFlag_Pause=1,
 AudioStreamingState_None=0, SyncMode_Wait=0, SyncMode_Count=-1 };
#define SCREEN_WIDTH 320
#define SCREEN_HEIGHT 240
#define FRAMEBUFFER_HEIGHT_PROGRESSIVE 224
#define FRAMEBUFFER_HEIGHT_INTERLACED 448
#define ORDERING_TABLE_SIZE 2048
#define TICKS_PER_SECOND 60
#define TIMESTEP_60_FPS Q12(1.0f/60.0f)
#define SECONDS_60_FPS(n) ((n)*60)
extern s32 g_Demo_FrameCount, g_WarmBootTimer, g_ActiveBufferIdx, g_TickCount;
extern s32 g_VBlanks, g_UncappedVBlanks, g_IntervalVBlanks, g_Demo_VideoPresentInterval;
extern q19_12 g_DeltaTime, g_DeltaTimeRaw, g_GravitySpeed, g_ScreenFadeTimestep;
extern GsOT_TAG g_OtTags0[2][16], g_OtTags1[2][ORDERING_TABLE_SIZE];
extern GsOT g_OrderingTable0[2], g_OrderingTable2[2];
extern s_FsImageDesc g_MainImg0, g_KonamiLogoImg, g_KcetLogoImg, g_Font16AtlasImg;
extern _Alignas(8) u8 port_fs_buffers[2][1024*1024], port_packets[2][131072];
extern _Alignas(8) u8 port_overlay_body[1024*1024], port_overlay_dynamic[1024*1024];
// PORT: Native buffers replace PS1 fixed file/packet addresses.
#define FS_BUFFER_0 ((void*)port_fs_buffers[0])
#define FS_BUFFER_1 ((void*)port_fs_buffers[1])
#define TEMP_MEMORY_ADDR ((s8*)port_packets[0])

// PORT: OT links remain 24-bit tokens, resolved through a native pointer registry.
u32 port_gpu_token(const void* p);
void* port_gpu_pointer(u32 token);
#undef setaddr
#define setaddr(p, address) (((P_TAG*)(p))->addr = port_gpu_token((const void*)(uintptr_t)(address)))
#undef setRGB0
#define setRGB0(p,r,g,b) ((p)->r0=(u8)(r), (p)->g0=(u8)(g), (p)->b0=(u8)(b))
#undef setWH
#define setWH(p,_w,_h) ((p)->w=(s16)(_w), (p)->h=(s16)(_h))
// PORT: Pack signed 16-bit coordinates with unsigned shifts, preserving PS1 bits without C UB.
#undef setXY0Fast
#define setXY0Fast(p,x,y) (*(u32*)&(p)->x0 = (u32)(u16)(x) | ((u32)(u16)(y)<<16))

void Fs_QueueInitialize(void);
s32 Fs_QueueGetLength(void);
void Fs_QueueUpdate(void);
void Fs_QueueStartRead(s32 file, void* buffer);
void Fs_QueueStartReadTim(s32 file, void* buffer, s_FsImageDesc* image);
void Fs_QueueWaitForEmpty(void);
void Fs_DecryptOverlay(s32* dst, const s32* src, s32 size);
void Screen_BackgroundImgDraw(s_FsImageDesc* image);
void Screen_Init(s32 width, bool interlaced);
void Screen_VSyncCallback(void);
void Screen_XyPositionSet(s32 x, s32 y);
void Screen_DisplayEnvXySet(DISPENV* env, s32 x, s32 y);
void Game_StateStepIncrement(s32 index);
void port_game_state_next(s32 next);
#define Game_StateSetNext(n) port_game_state_next(n)
#include "bodyprog/screen/screen_fade.h"
#include "screens/b_konami/b_konami.h"

void MainLoop(void);
void GameState_Init_Update(void);
#define PORT_OTHER_STATES(X) \
 X(GameState_KcetLogo_Update) X(GameState_MovieIntroFadeIn_Update) \
 X(GameState_AutoLoadSavegame_Update) X(GameState_MovieIntroAlternate_Update) \
 X(GameState_MovieIntro_Update) X(GameState_MainMenu_Update) \
 X(GameState_LoadSavegameScreen_Update) X(GameState_MovieOpening_Update) \
 X(GameState_LoadScreen_Update) X(GameState_InGame_Update) X(GameState_MapEvent_Update) \
 X(GameState_ExitMovie_Update) X(GameState_ItemScreens_Update) X(GameState_PaperMapScreen_Update) \
 X(GameState_DebugMoviePlayer_Update) X(GameState_Options_Update) \
 X(GameState_LoadStatusScreen_Update) X(GameState_LoadMapScreen_Update) X(GameState_Credits_Update)
#define DECLARE_STATE(name) void name(void);
PORT_OTHER_STATES(DECLARE_STATE)
#undef DECLARE_STATE
void ResetCallback(void);
int CdInit(void);
void SpuInit(void);
int VSync(int mode);
int VSyncCallback(void (*callback)(void));
void SetDispMask(int mask);
s32 Math_MulFixed(s32 a, s32 b, s32 shift);
void Joy_Update(void);
void Joy_Init(void);
void Joy_ReadP1(void);
void Joy_ControllerDataUpdate(void);
void Demo_ControllerDataUpdate(void);
void Demo_Update(void);
void Demo_GameRandSeedSet(void);
void Demo_PresentIntervalUpdate(void);
s32 MainLoop_ShouldWarmReset(void);
void Game_WarmBoot(void);
void MemCard_SysInit(void);
void MemCard_SysEnable(void);
void MemCard_InitStatus(void);
void MemCard_Update(void);
bool MemCard_ElementsUpdate(void);
void ItemScreen_TmdGsFCallInit(void);
void func_800890B8(void);
void SD_Init(void);
void SD_Call(s32 task);
s32 Sd_AudioStreamingCheck(void);
void Sd_TaskPoolExecute(void);
void func_80089090(s32 mode);
void func_80089128(void);
void func_8008D78C(void);
void WorldGfx_HarryCharaLoad(void);
void GameFs_BgItemLoad(void);
void Map_EffectTexturesLoad(s32 map);
void nullsub_800334C8(void);
// Rust-owned disc/rasterizer callbacks. No borrowed pointers survive these synchronous calls.
int port_read_file(u32 id, u32 bytes, u8* destination);
void port_pad_read(u8* destination);
void port_begin_ot(void);
void port_end_ot(void);
int port_store_vram(s32 x, s32 y, s32 w, s32 h, u16* destination);
int port_move_vram(s32 x, s32 y, s32 w, s32 h, s32 dx, s32 dy);
void port_spu_reset(void);
int port_spu_write(u16 offset, u16 value);
u16 port_spu_read(u16 offset);
int port_spu_transfer(u32 address, const u8* data, u32 count);
void port_load_vram(s32 x, s32 y, s32 w, s32 h, const u16* data);
void port_clear_vram(s32 x, s32 y, s32 w, s32 h, u8 r, u8 g, u8 b);
void port_draw_packet(const u32* words, u32 count);
void port_draw_env(s32 x, s32 y, s32 w, s32 h, s32 ox, s32 oy);
int port_present(s32 x, s32 y, s32 w, s32 h, s32 state, s32 step);
int port_run_game(void);
#endif
