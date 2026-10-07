/* SPDX-License-Identifier: GPL-3.0-only; signatures from pinned GPL C. */
void Bgm_CrossfadeToTrack(s32 bgmIdx);
void Game_FlashlightAttributesFix(void);
void Game_RadioNoiseReset(void);
void Game_RadioSoundStop(void);
void Particle_EnvironmentSet(s8 arg0, u32 arg1);
void Particle_SystemUpdate(s32 unused, e_MapIdx mapIdx, s32 arg3);
void Player_ControlFreeze(void);
void Player_ControlUnfreeze(bool setIdle);
void Player_EmptyWeaponHandSet(void);
void Player_MoveSpeedClear(void);
void Player_WeaponAttackRestore(void);
void func_8003A16C(void);

void Gfx_MapEnvSet(s32 arg0,s32 arg1);
