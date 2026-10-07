/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_BOOT_H
#define SH_BOOT_H
#include "common.h"
#include "overlay.h"
#include <psyq/libgte.h>
#include <psyq/libgpu.h>
#include "gte_native.h"
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
enum { PRIM_RECT=0x60, RECT_TEXTURE=4, RECT_BLEND=2,PRIM_POLY=0x20,PRIM_LINE=0x40,RECT_SIZE_1=8,RECT_SIZE_8=16,RECT_SIZE_16=24 };
enum {Sfx_MenuError=1304};
#define getTPageN(tp,abr,xn,yn) (((tp)<<7)|((abr)<<5)|((yn)<<4)|(xn))
#define setRECTFast(p,_x,_y,_w,_h) ((p)->x=(s16)(_x),(p)->y=(s16)(_y),(p)->w=(s16)(_w),(p)->h=(s16)(_h))
#define setCodeWord(p,c,rgb) (*(u32*)((u8*)(p)+4)=((u32)(c)<<24)|((u32)(rgb)&0xffffff))
#define setWHFast(p,_w,_h) (*(u32*)&(p)->w=(u32)(u16)(_w)|((u32)(u16)(_h)<<16))
#define setUV0AndClut(p,u,v,cx,cy) (*(u32*)&(p)->u0=(u32)(u)|((u32)(v)<<8)|((u32)(((cy)<<6)|((cx)>>4))<<16))
#define setRGBC0(p,r,g,b,c) (*(u32*)&(p)->r0=(u32)(r)|((u32)(g)<<8)|((u32)(b)<<16)|((u32)(c)<<24))
#define setRGBC1(p,r,g,b,c) (*(u32*)&(p)->r1=(u32)(r)|((u32)(g)<<8)|((u32)(b)<<16)|((u32)(c)<<24))
#define setRGBC2(p,r,g,b,c) (*(u32*)&(p)->r2=(u32)(r)|((u32)(g)<<8)|((u32)(b)<<16)|((u32)(c)<<24))
#define addPrimFast(ot,p,len) ((p)->tag=getaddr(ot)|((u32)(len)<<24),setaddr(ot,p))
#include "bodyprog/math/arithmetic.h"
#include "bodyprog/math/constants.h"
#include "bodyprog/math/fixed_point.h"
#include "main/fileinfo.h"

// PORT: Boot runtime records are native objects, never views of game-file bytes.
// Full gameplay records must be migrated separately; upstream assertions stay intact in the baseline gate.
#include "game_records.h"
#include "gpu_records.h"
#include "native_gameplay_records.h"
#include "screens/options.h"
#include "main/rng.h"
#include <string.h>
typedef struct { s_ControllerConfig controllerConfig;
    s32 screenPositionX,screenPositionY,soundType,volumeBgm,volumeSe,
        extraWeaponCtrl,brightness,vibrationEnabled,extraBloodColor,autoLoad,extraOptionsEnabled,
        extraViewCtrl,extraViewMode,extraRetreatTurn,extraWalkRunCtrl,extraAutoAiming,extraBulletAdjust; } PortOptions;
typedef struct { u8 tPage[2], u, v; s16 clutX, clutY; } s_FsImageDesc;
STATIC_ASSERT_SIZEOF(s_FsImageDesc, 8);
#include "native_asset_records.h"
STATIC_ASSERT_SIZEOF(SPRT, 20);
STATIC_ASSERT_SIZEOF(TILE, 16);
STATIC_ASSERT_SIZEOF(DR_TPAGE, 8);
typedef struct {
    s32 gameState, gameStatePrev, gameStateSteps[3];
    struct { u8 r, g, b; } background2dColor;
    s32 gsScreenWidth, gsScreenHeight;
    PortOptions config;
    s_Savegame savegame,autosave;
    s_ControllerData controllers[2];
    s_AnalogController rawController;
    s8 mapAnimIdx,bgmIdx,ambientIdx;
} PortGameWork;
typedef struct {
    s32 gameStateCounter, gameStateStepCounter, sysStateCounter, sysState;
    s32 sysFlags, bgmStatusFlags;
    s32 processFlags;
    bool enableHalfHeightGlyphs;
    s_PlayerWork playerWork;
    s_PlayerCombat playerCombat;
    GsCOORDINATE2 playerBoneCoords[18],npcBoneCoordBuffer[NPC_BONE_COUNT_MAX];
    s_SubCharacter npcs[NPC_COUNT_MAX];
    s32 unused_229C;
    bool enablePlayerMatchAnim;
    s32 loadingScreenIdx;
    q3_12 cameraAngleY;
    q3_12 cameraAngleZ;
    q19_12 cameraRadiusXz,cameraY;
    u8 playerStopFlags;
    s8 targetNpcIdx,npcIdxs[CHARA_GROUP_COUNT];
    q19_12 field_275C,field_2760,field_2764;
    q3_12 lightIntensity;
    GsCOORDINATE2 *lightBoneCoord,*lensFlareBoneCoord;
    VECTOR3 lightPosition;
    SVECTOR lightRotation;
    struct {bool isFlashlightOn;} field_2388;
    // PORT: Native gameplay scalars appended without changing existing FFI offsets.
    s32 field_228C[1],npcFlags;
    s8 npcFlagId;
    u16 charaGroupFlags[CHARA_GROUP_COUNT];
    s8 field_2349;
    u8 field_234A;
    s8 areaTransitionFlags;
    q3_12 bgmLayerVolumes[9];
    bool isMgsStringSet;
    u8 invItemLoadFlags;
    // PORT: Append the full native environment record without shifting earlier fields.
    s_SysWork_2388 gameplayEnvironment;
    s32 cutsceneBorderState;
} PortSysWork;
typedef PortGameWork PortGameWorkConst;
#define g_GameWorkPtr (&g_GameWork)
typedef s_ControllerData PortController;
extern PortGameWork g_GameWork;
extern PortSysWork g_SysWork;
extern PortGameWorkConst* g_GameWorkConst;
extern PortController* g_Controller0;
extern PortController* g_Controller1;
#define g_SavegamePtr (&g_GameWork.savegame)

enum { GameState_Init=0, GameState_KonamiLogo=1, GameState_KcetLogo=2,
 GameState_MovieIntroFadeIn=3, GameState_AutoLoadSavegame=4, GameState_MovieIntroAlternate=5, GameState_MovieIntro=6,
 GameState_MainLoadScreen=10, GameState_InGame=11, GameState_InventoryScreen=14,
 GameState_MainMenu=7, GameState_Unk16=22,
 GameState_LoadSavegameScreen=8,GameState_MovieOpening=9,GameState_OptionScreen=18,
 BgmStatusFlag_None=0, BgmStatusFlag_Pause=1,
 AudioStreamingState_None=0, SyncMode_Wait=0, SyncMode_Count=-1, SyncMode_Wait2=2, SyncMode_Wait8=8 };
enum {MainMenuEntry_Load=0,MainMenuEntry_Continue=1,MainMenuEntry_Start=2,MainMenuEntry_Option=3,MainMenuEntry_Extra=4,MainMenuEntry_Count=5,
MainMenuState_Start=0,MainMenuState_Main=1,MainMenuState_LoadGame=2,MainMenuState_DifficultySelector=3,MainMenuState_NewGameStart=4,
GameDifficulty_Easy=-1,GameDifficulty_Hard=1,Sfx_MenuMove=1305,Sfx_MenuStartGame=1281,Sfx_MenuConfirm=1307,Sfx_MenuCancel=1306,
PortMenuConstants_End=0};
#include "map_constants.h"
typedef s32 e_GameState;
#define STICK_DEADZONE 64
#define DEFAULT_MAP_MESSAGE_LENGTH 100
#define COLOR_RGBC(r,g,b,c) ((u32)(r)|((u32)(g)<<8)|((u32)(b)<<16)|((u32)(c)<<24))
// PORT: Preserve packed signed coordinates without undefined signed shifts.
#define Math_SetDVectorFast(v,x,y) (*(u32*)&(v)->vx=(u32)(u16)(x)|((u32)(u16)(y)<<16))
enum {SavegameEntryType_NoMemCard=0,SavegameEntryType_OutOfBlocks=4,SavegameEntryType_Save=8,
    MemCardGameProcessId_Load_Game=2,MemCardWorkResult_Success=1,AudioMode_Mono=1,AudioMode_Stereo=2};
#define INPUT_ACTION_COUNT 14
#define OPT_SOUND_VOLUME_MAX 128
#define OPT_VIBRATION_ENABLED 128
typedef struct {s32 type,deviceId,fileIdx,elementIdx;} PortMemCardEntry;
extern PortMemCardEntry* g_MemCard_ActiveMemCardSlotSaves;
extern s32 g_SelectedSaveSlotIdx,g_SlotElementSelectedIdx[2],g_SelectedDeviceId,g_SelectedFileIdx,g_Savegame_SelectedElementIdx;
PortMemCardEntry* MemCard_ActiveMemCardSlotGet(s32 slot);
void MemCard_ProcessSet(s32 process,s32 device,s32 file,s32 element);
s32 MemCard_LastMemCardResultGet(void);
void Settings_RestoreDefaults(void);
void Settings_RestoreControlDefaults(s32 index);
void Settings_ScreenAndVolUpdate(void);
void Sd_GlobalVolumeSet(s32 maximum,s32 music,s32 effects);
void GameFs_BgEtcGfxLoad(void);
void GameFs_StreamBinLoad(void);
void GameFs_TitleGfxSeek(void);
void GameFs_TitleGfxLoad(void);
void Fs_QueueStartSeek(s32 file);
void Demo_SequenceAdvance(s32 value);
void Demo_DemoDataRead(void);
s32 Game_StateStepSet(s32 index,s32 value);
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
extern s_FsImageDesc g_MainImg0, g_KonamiLogoImg, g_KcetLogoImg, g_Font16AtlasImg,g_MemCardWarningImg,g_TitleImg,g_ItemInspectionImg;
extern _Alignas(8) u8 port_fs_buffers[10][1024*1024], port_packets[2][131072];
extern _Alignas(8) u8 port_overlay_body[1024*1024], port_overlay_dynamic[1024*1024];
// PORT: Native buffers replace PS1 fixed file/packet addresses.
#define FS_BUFFER_0 ((void*)port_fs_buffers[0])
#define FS_BUFFER_1 ((void*)port_fs_buffers[1])
#define FS_BUFFER_3 ((void*)port_fs_buffers[3])
#define FS_BUFFER_4 ((void*)port_fs_buffers[4])
int port_player_map_anim_load(u32 file,s_AnmHeader* destination);
#define IMAGE_BUFFER_3 FS_BUFFER_3
#define FS_BUFFER_5 ((void*)port_fs_buffers[5])
#define FS_BUFFER_6 ((void*)port_fs_buffers[6])
#define FS_BUFFER_7 ((void*)port_fs_buffers[7])
#define FS_BUFFER_9 ((void*)port_fs_buffers[9])
#define TEMP_MEMORY_ADDR ((s8*)port_packets[0])
#define PSX_SCRATCH_ADDR(offset) (port_scratch+(offset))

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
#define setXY1Fast(p,x,y) (*(u32*)&(p)->x1 = (u32)(u16)(x) | ((u32)(u16)(y)<<16))
#define setXY2Fast(p,x,y) (*(u32*)&(p)->x2 = (u32)(u16)(x) | ((u32)(u16)(y)<<16))
#define setXY3Fast(p,x,y) (*(u32*)&(p)->x3 = (u32)(u16)(x) | ((u32)(u16)(y)<<16))
#undef setXY0
#define setXY0(p,x,y) ((p)->x0=(s16)(x),(p)->y0=(s16)(y))
#undef setXY4
#define setXY4(p,a,b,c,d,e,f,g,h) ((p)->x0=(s16)(a),(p)->y0=(s16)(b),(p)->x1=(s16)(c),(p)->y1=(s16)(d),(p)->x2=(s16)(e),(p)->y2=(s16)(f),(p)->x3=(s16)(g),(p)->y3=(s16)(h))

void Fs_QueueInitialize(void);
s32 Fs_QueueGetLength(void);
void Fs_QueueUpdate(void);
s32 Fs_QueueStartRead(s32 file, void* buffer);
s32 Fs_QueueStartReadTim(s32 file, void* buffer, s_FsImageDesc* image);
bool Fs_QueueIsEntryLoaded(s32 index);
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
 X(GameState_AutoLoadSavegame_Update) \
 X(GameState_LoadSavegameScreen_Update) \
 X(GameState_InGame_Update) X(GameState_MapEvent_Update) \
 X(GameState_ItemScreens_Update) X(GameState_PaperMapScreen_Update) \
 X(GameState_LoadStatusScreen_Update) X(GameState_LoadMapScreen_Update) X(GameState_Credits_Update)
#define DECLARE_STATE(name) void name(void);
PORT_OTHER_STATES(DECLARE_STATE)
DECLARE_STATE(GameState_LoadScreen_Update)
DECLARE_STATE(GameState_MovieIntroFadeIn_Update)
DECLARE_STATE(GameState_MovieIntroAlternate_Update)
DECLARE_STATE(GameState_MovieIntro_Update)
DECLARE_STATE(GameState_MovieOpening_Update)
DECLARE_STATE(GameState_ExitMovie_Update)
DECLARE_STATE(GameState_DebugMoviePlayer_Update)
DECLARE_STATE(GameState_MainMenu_Update)
DECLARE_STATE(GameState_Options_Update)
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
void port_pad_refresh(void);
void PadInitDirect(u8* first,u8* second);
void PadStartCom(void);
void ControllerData_AnalogToDigital(s_ControllerData* controller,bool analog);
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
int port_asset_load_native(u32 id,u8* destination,u32 kind);
// Returns 0 on success, 1 for a missing slot, 2 for invalid/corrupt/I/O failure.
int port_save_read(u32 slot,u8* destination,u32 bytes);
int port_save_write(u32 slot,const u8* source,u32 bytes);
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
int port_present(s32 x, s32 y, s32 w, s32 h, s32 state, s32 step, s32 rgb24);
int port_movie_begin(u32 file, u32 last_frame);
int port_movie_tick(void);
void port_movie_end(s32 skipped);
void open_main(s32 file,s16 last_frame);
void Screen_RectInterlacedClear(s16 x,s16 y,s16 w,s16 h,u8 r,u8 g,u8 b);
void Text_Debug_PositionSet(s16 x,s16 y);
void Text_Debug_Draw(const char* text);
void MainMenu_SelectedOptionIdxReset(void);
void MainMenu_FogUpdate(void);
void Screen_Refresh(s32 width,bool interlaced);
void GameBoot_InGameStartup(void);
void GameBoot_SavegameInitialize(s8 overlay,s32 difficulty);
void GameBoot_WorldInit(void);
void GameBoot_MapLoad(s32 map);
void MemCard_SysDisable(void);
void GameFs_SaveLoadBinLoad(void);
void GameFs_OptionBinLoad(void);
void SysWork_StateSetNext(s32 state);
void Fs_QueueReset(void);
extern s32 g_MemCard_SavegameCount;
#include "map.h"
s32 Math_Sin(s32 angle);
s32 Math_Cos(s32 angle);
void port_unimplemented(const char* name);
void Game_SavegameResetPlayer(void);
void Gfx_StringPositionSet(s32 x,s32 y);
void Gfx_StringColorSet(s16 color);
void Gfx_StringLayerIdxSet(s32 layer);
void Gfx_StringLayerIdxReset(void);
bool Gfx_StringDraw(const char* text,s32 length);
void Gfx_StringDrawInt(s32 width,s32 value);
void Game_TimerUpdate(void);
void Game_RadioSoundStop(void);
void Bgm_MenuUpdate(void);
void Sd_SfxPlay(s32 id,s32 pan,s32 volume);
void Gfx_Primitive2dTextureSet(s32 x,s32 y,s32 layer,s32 blend);
void Options_BrightnessMenu_LinesDraw(s32 brightness);
extern s_FsImageDesc g_BrightnessScreenImg0,g_BrightnessScreenImg1,g_ControllerButtonAtlasImg;
int port_run_game(void);
#endif
