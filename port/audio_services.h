/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_AUDIO_SERVICES_H
#define SH_AUDIO_SERVICES_H
extern _Alignas(8) u8 audio_cd_buffer[0x10000],audio_vab_headers[4][0x10000],audio_kdt_buffer[0x10000];
extern s32 g_RadioPitchState;
extern u32 g_FileXaLoc[16];
extern s_BgmLayerLimits audio_map0_limits;
extern s16 audio_map0_flags[8];
void audio_map0_tables(void);
void Bgm_SongChange(s32 bgmIdx);
int audio_layer_stop_probe(void);
s32 audio_timer_open(bool (*callback)(void));
void audio_timer_period(u32 period);
void audio_timer_start(void);
void audio_timer_stop(void);
void audio_timer_enable(bool enabled);
void audio_clock_vblank(void);
void audio_trace(const char* event,s32 value);
void audio_positional_trace(s32 id,s32 voice,s32 pan,s32 attenuation,s32 pitch);
void EnterCriticalSection(void);
void ExitCriticalSection(void);
u8 audio_sfx_play(u16 sfxId,q0_7 balance,q0_8 vol);
#define SyncMode_Wait3 3
#undef itob
#define itob(x) ((u8)((((x)/10)<<4)|((x)%10)))
#endif
