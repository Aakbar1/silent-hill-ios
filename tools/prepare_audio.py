"""Native, data-free audio driver preparation from the pinned GPL decomp."""
from pathlib import Path
import argparse
import importlib.util
import re
from prepare_gameplay import function
from prepare_maps import enumeration, initializer

NOTICE = '/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n'

# PORT: Explicit PS1 scalar narrowing; pinned original statements, by function.
NATIVE_EDITS = {
    'Bgm_LayersUpdate': {
        'Sd_MidiChannelsVolumeSet(i, bgmChannelVols[i]);': 'Sd_MidiChannelsVolumeSet((u8)(i), (u8)(bgmChannelVols[i]));',
        'bgmChannelVols[i] = curChannelVol;': 'bgmChannelVols[i] = (s8)(curChannelVol);',
        'layerVols[i] = curLayerVol;': 'layerVols[i] = (q3_12)(curLayerVol);',
    },
    'Note2Pitch': {
        'cent_offset = temp - (pitch_steps << 7);': 'cent_offset = (s16)(temp - (pitch_steps << 7));',
        'final_note  = note + pitch_steps;': 'final_note  = (s16)(note + pitch_steps);',
    },
    'SD_Call': {
        'SD_BranchCTRL(task);': 'SD_BranchCTRL((u16)(task));',
        'Sd_SetupBgmMidiChannels(task);': 'Sd_SetupBgmMidiChannels((u16)(task));',
        'Sd_SfxStopStep(task - 0x200);': 'Sd_SfxStopStep((u16)(task - 0x200));',
        'Sd_XaAudioPlayTaskAdd(task);': 'Sd_XaAudioPlayTaskAdd((u16)(task));',
        'audio_sfx_play(task, Q8(0.0f), Q8(0.0f));': 'audio_sfx_play((u16)(task), (q0_7)(Q8(0.0f)), (q0_8)(Q8(0.0f)));',
    },
    'SdGetSeqBeat': {
        'return -1;': 'return (u32)(-1);',
    },
    'SdSeqOpen': {
        'return i;': 'return (s16)(i);',
    },
    'SdSetMidiExpress': {
        'control_change(midi_ch + (seq_access_num * 16), 11, expression & 0x7F);': 'control_change((u8)(midi_ch + (seq_access_num * 16)), (u8)(11), (u8)(expression & 0x7F));',
    },
    'SdSetMidiPan': {
        'control_change(midi_ch + (seq_access_num * 16), 10, pan & 0x7F);': 'control_change((u8)(midi_ch + (seq_access_num * 16)), (u8)(10), (u8)(pan & 0x7F));',
    },
    'SdSetMidiVol': {
        'control_change(midi_ch + (seq_access_num * 16), 7, vol & 0x7F);': 'control_change((u8)(midi_ch + (seq_access_num * 16)), (u8)(7), (u8)(vol & 0x7F));',
    },
    'SdSetSerialVol': {
        'attr.cd.volume.left  = v_left;': 'attr.cd.volume.left  = (short)(v_left);',
        'attr.cd.volume.right = v_right;': 'attr.cd.volume.right = (short)(v_right);',
        'attr.ext.volume.left  = v_left;': 'attr.ext.volume.left  = (short)(v_left);',
        'attr.ext.volume.right = v_right;': 'attr.ext.volume.right = (short)(v_right);',
    },
    'SdSpuMallocWithStartAddr': {
        'return -1;': 'return (u32)(-1);',
    },
    'SdUtKeyOn': {
        'return vc;': 'return (s16)(vc);',
    },
    'SdUtKeyOnV': {
        'smf_port[vo].l_vol_C = l_vol;': 'smf_port[vo].l_vol_C = (u16)(l_vol);',
        'smf_port[vo].r_vol_E = l_vol;': 'smf_port[vo].r_vol_E = (u16)(l_vol);',
        'smf_port[vo].l_vol_C = (l_vol * voll) >> 7;': 'smf_port[vo].l_vol_C = (u16)((l_vol * voll) >> 7);',
        'smf_port[vo].note_6      = note_value >> 8;': 'smf_port[vo].note_6      = (u16)(note_value >> 8);',
        'smf_port[vo].pan_14 = pan;': 'smf_port[vo].pan_14 = (u16)(pan);',
        'smf_port[vo].prog_2      = prog;': 'smf_port[vo].prog_2      = (u8)(prog);',
        'smf_port[vo].r_vol_E = (r_vol * volr) >> 7;': 'smf_port[vo].r_vol_E = (u16)((r_vol * volr) >> 7);',
        'smf_port[vo].vab_id_52 = vabid;': 'smf_port[vo].vab_id_52 = (u8)(vabid);',
        'smf_port[vo].vc_0        = vo;': 'smf_port[vo].vc_0        = (u8)(vo);',
        'return vo;': 'return (s16)(vo);',
    },
    'SdVabTransBody': {
        'return vab_h_id;': 'return (s16)(vab_h_id);',
    },
    'SdVabTransBodyPartly': {
        'return retval;': 'return (s16)(retval);',
    },
    'SdVabTransCompleted': {
        'return SpuIsTransferCompleted(0);': 'return (s16)(SpuIsTransferCompleted(0));',
        'return SpuIsTransferCompleted(1);': 'return (s16)(SpuIsTransferCompleted(1));',
    },
    'SdVbKeyOn': {
        'smf_port[voice].center_1E = center;': 'smf_port[voice].center_1E = (u8)(center);',
        'smf_port[voice].shift_1F  = shift;': 'smf_port[voice].shift_1F  = (u8)(shift);',
        'smf_port[voice].vab_id_52 = vabid;': 'smf_port[voice].vab_id_52 = (u8)(vabid);',
        'smf_port[voice].vc_0      = voice;': 'smf_port[voice].vc_0      = (u8)(voice);',
        'return voice;': 'return (s16)(voice);',
    },
    'SdVoKeyOn': {
        'vc = SdUtKeyOnV((voice >> 16), vabid, prog, tone, note, pitch & 0xFF, voll, volr);': 'vc = SdUtKeyOnV((s16)((voice >> 16)), (s16)(vabid), (s16)(prog), (s16)(tone), (s16)(note), (s16)(pitch & 0xFF), (s16)(voll), (s16)(volr));',
        'vabid = vab_pro >> 8;': 'vabid = (s16)(vab_pro >> 8);',
    },
    'Sd_AllSfxStop': {
        'SdUtKeyOffV(i);': 'SdUtKeyOffV((s16)(i));',
    },
    'Sd_AllSfxWithRRStop': {
        'SdUtKeyOffVWithRROff(i);': 'SdUtKeyOffVWithRROff((s16)(i));',
    },
    'Sd_AudioStop': {
        'SdVabClose(i);': 'SdVabClose((s16)(i));',
    },
    'Sd_BgmStopTaskAdd': {
        'g_Sd_AudioWork.midiChannelsVolTask = NO_VALUE;': 'g_Sd_AudioWork.midiChannelsVolTask = (u16)(NO_VALUE);',
    },
    'Sd_CdPrimitiveCmdTry': {
        'comCpy = com;': 'comCpy = (u8)(com);',
    },
    'Sd_GlobalVolumeSet': {
        'gSDVolConfig.globalVolumeBgm = bgmVol;': 'gSDVolConfig.globalVolumeBgm = (u8)(bgmVol);',
        'gSDVolConfig.globalVolumeSe  = seVol;': 'gSDVolConfig.globalVolumeSe  = (u8)(seVol);',
        'gSDVolConfig.globalVolumeXa  = xaVol;': 'gSDVolConfig.globalVolumeXa  = (u8)(xaVol);',
    },
    'Sd_KdtLoad_TaskAdd': {
        'Sd_TaskPoolAdd(task);': 'Sd_TaskPoolAdd((u8)(task));',
    },
    'Sd_MidiChannelVolumeGet': {
        'vol = SdGetMidiVol(0, i);': 'vol = SdGetMidiVol((s16)(0), (s16)(i));',
    },
    'Sd_MidiChannelsVolumeSet': {
        'SdSetMidiVol(0, i, volCpy);': 'SdSetMidiVol((s16)(0), (s16)(i), volCpy);',
    },
    'Sd_SetReverbDepth': {
        'SdUtSetReverbDepth(depthCpy, depthCpy);': 'SdUtSetReverbDepth((s16)(depthCpy), (s16)(depthCpy));',
    },
    'Sd_SetReverbEnable': {
        'SdSetSerialAttr(0, 1, mode);': 'SdSetSerialAttr((char)(0), (char)(1), (char)(mode));',
    },
    'Sd_SfxWithPitchPlay': {
        'g_Sd_VabPlayingInfo.voiceIdx = SdUtKeyOn(g_Sd_VabPlayingInfo.typeIdx, g_Sd_VabPlayingInfo.progIdx, g_Sd_VabPlayingInfo.toneIdx, g_Sd_VabPlayingInfo.noteIdx, g_Sd_VabPlayingInfo.pitch,\n                                             Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeLeft), Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeRight));': 'g_Sd_VabPlayingInfo.voiceIdx = (u8)(SdUtKeyOn(g_Sd_VabPlayingInfo.typeIdx, g_Sd_VabPlayingInfo.progIdx, g_Sd_VabPlayingInfo.toneIdx, g_Sd_VabPlayingInfo.noteIdx, g_Sd_VabPlayingInfo.pitch,\n                                             Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeLeft), Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeRight)));',
    },
    'Sd_TaskPoolExecute': {
        'g_Sd_AudioWork.midiChannelsVolTask = NO_VALUE;': 'g_Sd_AudioWork.midiChannelsVolTask = (u16)(NO_VALUE);',
    },
    'Sd_VabLoad_TaskAdd': {
        'Sd_TaskPoolAdd(task);': 'Sd_TaskPoolAdd((u8)(task));',
    },
    'Sd_XaAudioPlay': {
        'g_Sd_XaCdlInfo.cdlFilter.chan         = gSDXATable[xaAudioIdx].field_4_24;': 'g_Sd_XaCdlInfo.cdlFilter.chan         = (u_char)(gSDXATable[xaAudioIdx].field_4_24);',
        'g_Sd_XaCdlInfo.cdlFilter.file         = gSDXATable[xaAudioIdx].field_8_24;': 'g_Sd_XaCdlInfo.cdlFilter.file         = (u_char)(gSDXATable[xaAudioIdx].field_8_24);',
    },
    'Sd_XaPreLoadAudio': {
        'g_Sd_XaCdlInfo.cdlFilter.chan            = gSDXATable[xaAudioIdx].field_4_24;': 'g_Sd_XaCdlInfo.cdlFilter.chan            = (u_char)(gSDXATable[xaAudioIdx].field_4_24);',
        'g_Sd_XaCdlInfo.cdlFilter.file            = gSDXATable[xaAudioIdx].field_8_24;': 'g_Sd_XaCdlInfo.cdlFilter.file            = (u_char)(gSDXATable[xaAudioIdx].field_8_24);',
    },
    'audio_sfx_play': {
        'g_Sd_VabPlayingInfo.voiceIdx = SdVoKeyOn(g_Vab_InfoTable[audioIdx].vabProgIdx, g_Sd_VabPlayingInfo.noteIdx * 0x100,\n                                                 Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeLeft), Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeRight));': 'g_Sd_VabPlayingInfo.voiceIdx = (u8)(SdVoKeyOn(g_Vab_InfoTable[audioIdx].vabProgIdx, g_Sd_VabPlayingInfo.noteIdx * 0x100,\n                                                 Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeLeft), Sd_SeVolumeGet(g_Sd_VabPlayingInfo.volumeRight)));',
        'return NO_VALUE;': 'return (u8)(NO_VALUE);',
    },
    'egetc': {
        'return -1;': 'return (u32)(-1);',
    },
    'key_on': {
        'temp_s0_5 += (port->note_wk_8 << 7) + pitch_bend_calc(port, midi->pbend_7);': 'temp_s0_5 = (s16)(temp_s0_5 + ((port->note_wk_8 << 7) + pitch_bend_calc(port, midi->pbend_7)));',
        'port->a_mode_4A      = s_attr.a_mode;': 'port->a_mode_4A      = (u16)(s_attr.a_mode);',
        'port->prog_2         = progNo;': 'port->prog_2         = (u8)(progNo);',
        'port->vc_0           = vo;': 'port->vc_0           = (u8)(vo);',
    },
    'metaevent': {
        'smf_song[smf_file_no].tracks_0[a].mf_tempo2_16 = tempo;': 'smf_song[smf_file_no].tracks_0[a].mf_tempo2_16 = (u16)(tempo);',
        'smf_song[smf_file_no].tracks_0[a].mf_tempo_14  = tempo;': 'smf_song[smf_file_no].tracks_0[a].mf_tempo_14  = (u16)(tempo);',
    },
    'midi_file_out': {
        'p->mf_delta_time_1C       = readvarinum(p);': 'p->mf_delta_time_1C       = (u16)(readvarinum(p));',
    },
    'midi_smf_main': {
        'p->mf_delta_time_1C = readvarinum(p);': 'p->mf_delta_time_1C = (u16)(readvarinum(p));',
    },
    'midi_vsync': {
        'replay_reverb_set(i);': 'replay_reverb_set((s16)(i));',
    },
    'pitch_bend_calc': {
        'pitch = (short)(p->bend_max_1C * bendMultiplier) * (pit - 0x3F);': 'pitch = (s16)((short)(p->bend_max_1C * bendMultiplier) * (pit - 0x3F));',
    },
    'pitch_calc': {
        'pitch += (p->note_wk_8 << 7) + pitch_bend_calc(p, m->pbend_7);': 'pitch = (s16)(pitch + ((p->note_wk_8 << 7) + pitch_bend_calc(p, m->pbend_7)));',
    },
    'random_calc': {
        'p->rdmd_40 = (rd_data >> 2);': 'p->rdmd_40 = (u16)((rd_data >> 2));',
    },
    'read16bit': {
        'return to16bit(c1, c2);': 'return (u16)(to16bit(c1, c2));',
        's8 c1 = egetc(p);': 's8 c1 = (s8)(egetc(p));',
        's8 c2 = egetc(p);': 's8 c2 = (s8)(egetc(p));',
    },
    'read32bit': {
        's8 c1 = egetc(p);': 's8 c1 = (s8)(egetc(p));',
        's8 c2 = egetc(p);': 's8 c2 = (s8)(egetc(p));',
        's8 c3 = egetc(p);': 's8 c3 = (s8)(egetc(p));',
        's8 c4 = egetc(p);': 's8 c4 = (s8)(egetc(p));',
    },
    'readEOF': {
        'return -1;': 'return (u32)(-1);',
    },
    'readMThd': {
        'return -1;': 'return (u32)(-1);',
    },
    'readMTrk': {
        'return -1;': 'return (u32)(-1);',
    },
    'readheader': {
        'smf_song[file_no].mf_division_528 = seqh->tb_8;': 'smf_song[file_no].mf_division_528 = (u16)(seqh->tb_8);',
        'smf_song[file_no].mf_head_len_52A = read32bit(p);': 'smf_song[file_no].mf_head_len_52A = (u16)(read32bit(p));',
        'smf_song[file_no].mf_tracks_526   = seqh->tracks_C;': 'smf_song[file_no].mf_tracks_526   = (u16)(seqh->tracks_C);',
    },
    'readtrack': {
        'c = egetc(p);': 'c = (u8)(egetc(p));',
        'c1 = egetc(p);': 'c1 = (u8)(egetc(p));',
        'c2 = egetc(p);': 'c2 = (u8)(egetc(p));',
        'type = egetc(p);': 'type = (u8)(egetc(p));',
    },
    'readtrack2': {
        'smf_song[(p->midi_ch_27 >> 4)].tracks_0[tr].mf_tempo2_16 = tempo;': 'smf_song[(p->midi_ch_27 >> 4)].tracks_0[tr].mf_tempo2_16 = (u16)(tempo);',
        'c1 = egetc(p);': 'c1 = (s8)(egetc(p));',
        'c = egetc(p);': 'c = (u8)(egetc(p));',
    },
    'smf_vol_set': {
        's_attr.volume.left  = (l_vol * smf_song[ch >> 4].sd_seq_mvoll_50C) >> 7;': 's_attr.volume.left  = (short)((l_vol * smf_song[ch >> 4].sd_seq_mvoll_50C) >> 7);',
        's_attr.volume.right = (r_vol * smf_song[ch >> 4].sd_seq_mvolr_50E) >> 7;': 's_attr.volume.right = (short)((r_vol * smf_song[ch >> 4].sd_seq_mvolr_50E) >> 7);',
    },
    'sound_off': {
        'smf_port[vo].vc_0        = vo;': 'smf_port[vo].vc_0        = (u8)(vo);',
    },
    'sound_seq_off': {
        'port->vc_0        = vo;': 'port->vc_0        = (u8)(vo);',
        'track->midi_ch_27             = access_num * 0x10;': 'track->midi_ch_27             = (u8)(access_num * 0x10);',
    },
    'tre_calc': {
        'p->tre_data_3E = tre_data >> 8;': 'p->tre_data_3E = (s16)(tre_data >> 8);',
        's_attr.volume.left = vol;': 's_attr.volume.left = (short)(vol);',
        's_attr.volume.right = vol;': 's_attr.volume.right = (short)(vol);',
    },
    'vib_calc': {
        'p->vib_data_2E = vib_data >> 10;': 'p->vib_data_2E = (u16)(vib_data >> 10);',
    },
    'volume_calc': {
        'p->l_vol_C = (l_vol * (p->velo_1A & 0x7F)) >> 7;': 'p->l_vol_C = (u16)((l_vol * (p->velo_1A & 0x7F)) >> 7);',
        'p->pan_14 = pan;': 'p->pan_14 = (u16)(pan);',
        'p->r_vol_E = (r_vol * (p->velo_1A & 0x7F)) >> 7;': 'p->r_vol_E = (u16)((r_vol * (p->velo_1A & 0x7F)) >> 7);',
    },
}


def prepare(decomp, out):
    out.mkdir(parents=True, exist_ok=True)
    read = lambda p: (decomp / p).read_text(encoding='utf-8')
    module = Path(__file__).resolve().parents[1] / 'crates/psxspu/port/prepare_libsd.py'
    spec = importlib.util.spec_from_file_location('prepare_libsd', module)
    fixes = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixes)
    for name in ('libspu.h', 'libsnd.h', 'libcd.h'):
        text = re.sub(r'\bunsigned\s+long\b','uint32_t',read('include/psyq/' + name))
        text = re.sub(r'\blong\b', 'int32_t', text)
        # PORT: common.h supplies fixed-width PsyQ aliases on native targets.
        text=text.replace('#include <sys/types.h>', '#include "common.h"')
        (out / 'psyq' / name).write_text(NOTICE + text, encoding='utf-8')
    libsd = read('include/bodyprog/libsd.h')
    # PORT: Allocator lookahead/compaction reads slot 16. A zero sentinel
    # replaces the original reliance on adjacent BSS without adding a slot.
    libsd=libsd.replace('sd_spu_alloc[SD_ALLOC_SLOTS]', 'sd_spu_alloc[SD_ALLOC_SLOTS + 1]')
    # PORT: These are native work records, never disk overlays. Keep disk records
    # and explicit native pointer-layout assertions; do not suppress assertions.
    libsd = libsd.replace('STATIC_ASSERT_SIZEOF(VAB_H, 28);',
        'STATIC_ASSERT_SIZEOF(VAB_H, sizeof(void*) == 8 ? 48 : 28);')
    libsd = libsd.replace('STATIC_ASSERT_SIZEOF(SMF_SONG, 1340);',
        'STATIC_ASSERT_SIZEOF(SMF_SONG, sizeof(void*) == 8 ? 1360 : 1340);')
    sound = read('include/bodyprog/sound/sound_system.h')
    sound = sound.replace('#define SD_TASK_CHANNEL_SET(idx)', '#undef SD_TASK_CHANNEL_SET\n#define SD_TASK_CHANNEL_SET(idx)')
    # boot.h already declares these public seams with the native caller ABI.
    for name in ('SD_Call','Sd_AudioStreamingCheck','Sd_GlobalVolumeSet','Sd_SfxPlay'):
        sound = re.sub(r'^.*\b'+name+r'\([^;]*;', '', sound, flags=re.M)
    sound = re.sub(r'AudioMode_Mono\s*=\s*1,', 'AudioMode_NativeMono = 1,', sound)
    sound = re.sub(r'AudioMode_Stereo\s*=\s*2', 'AudioMode_NativeStereo = 2', sound)
    sound = re.sub(r'AudioStreamingState_None\s*=\s*0,', 'AudioStreamingState_NativeNone = 0,', sound)
    # PORT: MSVC mixed-base bitfields allocate separate storage; use u32 for the
    # entire disk word and retain the original 12-byte assertion.
    sound = re.sub(r'u8(\s+field_[48]_24\s*:)', r'u32\1', sound)
    bgm = read('include/bodyprog/events/bgm_update.h')
    bgm = re.sub(r'BgmStatusFlag_None\s*=\s*0,', 'BgmStatusFlag_NativeNone = 0,', bgm)
    bgm = re.sub(r'BgmStatusFlag_Pause\s*=\s*1 << 0,', 'BgmStatusFlag_NativePause = 1 << 0,', bgm)
    # BgmCmd is already in the shared map constants.
    bgm = bgm.replace(enumeration(bgm, 'BgmCmd'), '')
    (out / 'audio_records.h').write_text(NOTICE+'#ifndef SH_AUDIO_RECORDS_H\n#define SH_AUDIO_RECORDS_H\n#include "npc_startup.h"\n#include <psyq/libspu.h>\n#include <psyq/libcd.h>\n#define BSS_HACK_SD_CALL_C\n#define PAD_HACK_IGNORE\n#define STATIC_ASSERT(c,n) _Static_assert(c,#n)\n#define ARRAY_SIZE(a) (sizeof(a)/sizeof((a)[0]))\ntypedef s8 q0_7;\ntypedef s32 q23_8;\n'+libsd+'\n'+sound+'\n'+bgm+'\n#include "audio_services.h"\n#endif\n',encoding='utf-8')
    sources=[]
    for name in ('smf_main','smf_mid','smf_io','smf_snd'):
        text = read('src/bodyprog/libsd/'+name+'.c')
        if name+'.c' in fixes.EDITS:
            text = fixes.transform(name+'.c',text)
        text = re.sub(r'^#include.*$', '', text, flags=re.M)
        if name=='smf_main':
            # PORT: Native timer services retain the original start/stop/order.
            text = text.replace('OpenEvent(RCntCNT2, EvSpINT, EvMdINTR, smf_timer)', 'audio_timer_open(smf_timer)')
            text = text.replace('SetRCnt(RCntCNT2, 7328, RCntMdINTR)', 'audio_timer_period(7328)')
            for a,b in {'StartRCnt(RCntCNT2)':'audio_timer_start()', 'StopRCnt(RCntCNT2)':'audio_timer_stop()', 'EnableEvent(sd_timer_event)':'audio_timer_enable(true)', 'DisableEvent(sd_timer_event)':'audio_timer_enable(false)', 'CloseEvent(sd_timer_event)':'audio_timer_enable(false)'}.items(): text=text.replace(a,b)
            text=text.replace('char eof_char[3]', 'u8 eof_char[3]')
        if name=='smf_mid': text=text.replace('extern char eof_char[3]', 'extern u8 eof_char[3]')
        sources.append(text)
    sources.insert(0, read('src/bodyprog/libsd/smf_tables.h'))
    sources=[s.replace('sd_spu_alloc[SD_ALLOC_SLOTS]', 'sd_spu_alloc[SD_ALLOC_SLOTS + 1]') for s in sources]
    sd = re.sub(r'^#include.*$', '', read('src/bodyprog/sound/sd_call.c'),flags=re.M)
    sd = sd.replace('void SD_Call(u32 task)', 'void SD_Call(s32 task)')
    sd = sd.replace('u8 Sd_AudioStreamingCheck(void)', 's32 Sd_AudioStreamingCheck(void)')
    sd = sd.replace('u8 Sd_SfxPlay(u16 sfxId, q0_7 balance, q0_8 vol)', 'void Sd_SfxPlay(s32 sfxId, s32 balance, s32 vol)')
    # Adapt the widened public wrapper return while retaining the driver internally.
    sd = sd.replace('void Sd_SfxPlay(s32 sfxId, s32 balance, s32 vol)', 'u8 audio_sfx_play(u16 sfxId, q0_7 balance, q0_8 vol)')
    sd = re.sub(r'\bSd_SfxPlay\(', 'audio_sfx_play(', sd)
    sd = sd.replace('void Sd_GlobalVolumeSet(u8 xaVol, s16 bgmVol, u8 seVol)', 'void Sd_GlobalVolumeSet(s32 xaVol, s32 bgmVol, s32 seVol)')
    sd = sd.replace('FS_BUFFER_1', 'audio_cd_buffer').replace('CD_ADDR_0','audio_cd_buffer')
    sd = sd.replace('SdSeqOpen(g_Sd_KdtBuffer[g_Sd_AudioType], 3)', 'SdSeqOpen((s32*)g_Sd_KdtBuffer[g_Sd_AudioType], 3)')
    sd = sd.replace('            char unk = -unk;', '            // PORT: Matching-only uninitialized local has no observable effect.')
    sources.append(sd)
    data=re.sub(r'^#include.*$','',read('src/bodyprog/sound/sound_data.c'),flags=re.M)
    for addr,idx in [('0x801FE460',0),('0x801FD840',1),('0x801FC220',2),('0x801FA600',3)]:
        data=data.replace('(u8*)'+addr, 'audio_vab_headers['+str(idx)+']')
    data=data.replace('(u8*)0x801F5600','audio_kdt_buffer')
    sources.append(data)
    sources.append(initializer(read('src/main/fileinfo.c'),'g_FileXaLoc'))
    original=read('src/bodyprog/events/bgm_update.c')
    for name in ('Bgm_Update','Bgm_MuteCheck','Bgm_LayerGlobalVariablesUpdate','Bgm_LayersUpdate','Bgm_MenuUpdate','Bgm_SongChange'):
        sources.append(function(original,name).replace('channelLimitsCpy = layerLimits','channelLimitsCpy = (u8*)layerLimits'))
    prelude='static s32 g_Bgm_LayersUpdated;\nstatic s32 g_Bgm_ChannelSetProcessState;\nstatic u8 g_Bgm_ChannelLimits[8]={128,128,128,128,128,128,128,128};\nstatic void Bgm_LayerGlobalVariablesUpdate(void);\n'
    forwards='\n'.join(m[1]+';' for m in re.finditer(r'^(static (?:inline )?\w+ \w+\([^;{}]*\))[^\n]*\n?\{',sd,re.M))
    code=NOTICE+'#include "audio_records.h"\n'+forwards+'\n'+prelude+'\n'.join(sources)
    for name in ['Sfx_Base','Sfx_RadioInterferenceLoop','Sfx_RadioStaticLoop']:
        match=re.search(r'\b'+name+r'\s*=\s*([^,\n]+)',read('include/bodyprog/sound/sfx_id_enum.h'))
        expression=match[1].replace('Sfx_Base','1280')
        code=re.sub(r'\b'+name+r'\b','('+expression+')',code)
    value=re.search(r'ItemToggleFlag_RadioOn\s*=\s*([^,\n]+)',read('include/bodyprog/items.h'))[1]
    code=code.replace('ItemToggleFlag_RadioOn','('+value+')')
    code=code.replace('&sd_vb_malloc_rec','(char*)sd_vb_malloc_rec')
    code=code.replace('MemCmp("MThd"','MemCmp((u8*)"MThd"').replace('MemCmp("MTrk"','MemCmp((u8*)"MTrk"')
    code=code.replace('&g_Sd_XaCdlInfo.cdlFilter','(u8*)&g_Sd_XaCdlInfo.cdlFilter').replace('CdlSeekL, &g_Sd_XaCdLocation','CdlSeekL, (u8*)&g_Sd_XaCdLocation')
    code=re.sub(r'CdRead\(([^;]+), audio_cd_buffer, 128\)',r'CdRead(\1, (u32*)audio_cd_buffer, 128)',code)
    code=code.replace('s32* ptr;','u32* ptr;')
    code=code.replace('g_MapOverlayHdr.bgmCmd = bgmIdx', '((s_MapOverlayHdr*)port_map_active())->bgmCmd = (s8)bgmIdx')
    # PORT: Upstream bool is a 32-bit enum. This temporary first holds the
    # complete channel-task ID, including 0xffff, before becoming a predicate.
    # Native _Bool would collapse the stop sentinel to 1 and change the logic.
    code=code.replace('bool      areChannelsActive;', 's32       areChannelsActive;')
    # PORT: Keep the upstream nullsubs while explicitly discarding arguments.
    def unused(m):
        names=re.findall(r'\b(\w+)\s*(?:,|$)',m[2])
        names=[n for n in names if n!='void']
        return m[1]+m[2]+') {'+''.join('(void)'+n+';' for n in names)+'}'
    code=re.sub(r'(void \w+\()([^{};]+)\) \{\}',unused,code)
    # Local SDK attributes shadow the driver's global s_attr in upstream C.
    # Rename within the owning functions without changing global consumers.
    for m in list(re.finditer(r'^\w+ \w+\([^;{}]*\)[^{;]*\{',code,re.M)):
        name=re.search(r'\b(\w+)\(',m[0])[1]
        body=function(code,name)
        if re.search(r'SpuVoiceAttr\s+s_attr;',body): code=code.replace(body,re.sub(r'\bs_attr\b','voice_attr',body))
    # PORT: Preserve C's original unsigned comparison/arithmetic conversions.
    code=code.replace('smf_song[smf_file_no].mf_data_size_518 <', '(u32)smf_song[smf_file_no].mf_data_size_518 <')
    code=code.replace('0x80000 - sd_reverb_area_size[sd_reverb_mode]', '(u32)(0x80000 - sd_reverb_area_size[sd_reverb_mode])')
    code=code.replace('!= p->vb_size_14', '!= (u32)p->vb_size_14').replace('== vab_h[vabid].vb_size_14','== (u32)vab_h[vabid].vb_size_14')
    code=code.replace('body_partly_size < vab_h[vabid].vb_size_14', 'body_partly_size < (u32)vab_h[vabid].vb_size_14')
    code=code.replace('temp_v1 == seq_access_num', 'temp_v1 == (u32)seq_access_num')
    code=code.replace('i < g_Sd_KdtTargetLoad->fileSize', '(u32)i < g_Sd_KdtTargetLoad->fileSize')
    code=code.replace('return -1u;', 'return -1;')
    code=code.replace('pitch = -((p->bend_min_1D * bendMultiplier) * (0x40 - pit));','pitch = (s16)(0u-((p->bend_min_1D * bendMultiplier) * (0x40 - pit)));')
    code=code.replace('get_vab_tone(p, i, chan)', 'get_vab_tone(p, (u16)i, chan)')
    code=code.replace('Sd_MidiChannelVolumeGet(i)', 'Sd_MidiChannelVolumeGet((u8)i)')
    body=function(code,'SdVoKeyOn')
    code=code.replace(body,re.sub(r'\bs32\s+vc;', 's32 vc = -1; // PORT: No matching tone reports failure instead of an uninitialized voice.',body))
    code=code.replace('    SpuReverbAttr rev_attr; // Used in `AUDIO.IRX` but unused here.', '')
    for name,arg in [('key_off','c2'),('pitch_bend','c1'),('SdUtAllKeyOff','mode')]:
        body=function(code,name)
        code=code.replace(body,body.replace('{','{\n    (void)'+arg+';',1))
    for name,changes in NATIVE_EDITS.items():
        body=function(code,name)
        adapted=body
        for before,after in changes.items():
            if before not in body: raise ValueError(f'{name}: pinned scalar statement changed: {before}')
            adapted=adapted.replace(before,after)
        code=code.replace(body,adapted)
    code += '\nvoid Sd_SfxPlay(s32 id,s32 pan,s32 vol) { (void)audio_sfx_play((u16)id,(q0_7)pan,(q0_8)vol); }\n'
    code += '''
// PORT: Audio-only regression fixture for upstream enum-bool task storage.
// Restore every touched controller field; never substitute a gameplay update.
int audio_layer_stop_probe(void) {
    q3_12 volumes[9];memcpy(volumes,g_SysWork.bgmLayerVolumes,sizeof(volumes));
    s32 state=g_Bgm_ChannelSetProcessState,updated=g_Bgm_LayersUpdated,flags=g_SysWork.bgmStatusFlags;
    u16 task=g_Sd_AudioWork.midiChannelsVolTask;
    g_Sd_AudioWork.midiChannelsVolTask=0xffff;
    g_Bgm_ChannelSetProcessState=1;
    Bgm_LayerGlobalVariablesMute();g_SysWork.bgmLayerVolumes[0]=4096;
    Bgm_LayersUpdate(BgmFlag_KeepAlive|BgmFlag_Layer1,0,NULL);
    int ok=g_SysWork.bgmLayerVolumes[0]==0 && g_Bgm_ChannelSetProcessState==0 && Sd_MidiChannelTaskGet()==0xffff;
    memcpy(g_SysWork.bgmLayerVolumes,volumes,sizeof(volumes));
    g_Bgm_ChannelSetProcessState=state;g_Bgm_LayersUpdated=updated;
    g_Sd_AudioWork.midiChannelsVolTask=task;
    g_SysWork.bgmStatusFlags=flags;
    return ok;
}
'''
    for name in ['SD_Init','SD_Call','Sd_VabLoad_Finalization','Sd_KdtLoad_LoadCheck','func_80046A70','Bgm_Update']:
        body=function(code,name)
        trace='audio_trace("'+name+'",'+('task' if name=='SD_Call' else '0')+');'
        code=code.replace(body,body.replace('{','{\n    '+trace,1))
    (out/'audio_driver.c').write_text(code,encoding='utf-8')
    # Compile copied caller sources with only obsolete sound definitions renamed.
    # No edits to move-owned sources or its generator; future removals are safe.
    root=Path(__file__).resolve().parents[1]
    replacements={'runtime.c':['SD_Init','SD_Call','Sd_TaskPoolExecute','Sd_AudioStreamingCheck','Sd_GlobalVolumeSet','SpuInit'],
                  'title_services.c':['Bgm_MenuUpdate','Sd_SfxPlay'],
                  'npc_startup.c':['Bgm_Update','Sd_MidiChannelTaskGet'],
                  'gameplay.c':['Sd_SfxStop']}
    for filename,names in replacements.items():
        text=(root/'port'/filename).read_text(encoding='utf-8')
        # Only rename definitions, leaving all callers bound to real services.
        for name in names:
            text=re.sub(r'(?m)^(void|s32|u16) '+name+r'\(',r'\1 audio_previous_'+name+'(',text)
            text=text.replace('STUB0('+name+')','STUB0(audio_previous_'+name+')').replace('STUB1('+name+')','STUB1(audio_previous_'+name+')')
            text=text.replace('STUB_RETURN('+name+',','STUB_RETURN(audio_previous_'+name+',')
        if filename=='runtime.c':
            text=text.replace('    if (vsync_callback) vsync_callback();', '    audio_clock_vblank();\n    if (vsync_callback) vsync_callback();')
            # PORT: Restore STREAM's original CD mix/volume writes around the
            # native movie service; feeding PCM must never enable mixing itself.
            text=text.replace('    port_movie_end(skipped);','    port_movie_end(skipped);\n    SsSetSerialVol(0,0,0);')
            text=text.replace('    Fs_QueueWaitForEmpty();\n    Screen_RectInterlacedClear(0,16,480,480,0,0,0);','    Fs_QueueWaitForEmpty();\n    SsSetSerialAttr(0,0,1);\n    SsSetSerialVol(0,80,80);\n    Screen_RectInterlacedClear(0,16,480,480,0,0,0);')
            text=text.replace('    blocked_service=NULL;', '    audio_clock_reset();\n    blocked_service=NULL;')
            text='void audio_clock_vblank(void);\nvoid audio_clock_reset(void);\nvoid SsSetSerialAttr(char,char,char);\nvoid SsSetSerialVol(char,short,short);\n'+text
        (out/('audio_caller_'+filename)).write_text(NOTICE+text,encoding='utf-8')
    # Supply the original map BGM callback, which currently has a guard.
    mapfile=out/'map0_s00.c'
    text=mapfile.read_text(encoding='utf-8')
    name='Map_RoomBgmInit'
    code=function(read('src/maps/map0_s00/map0_s00_2.c'),name)
    # Existing guard has the map-specific namespace; preserve the descriptor.
    matches=re.findall(r'void (\w*'+name+r')\([^{}]*\)\s*\{[^{}]*port_unimplemented[^{}]*\}',text)
    if len(matches)!=1: raise ValueError('expected one MAP0_S00 BGM guard')
    namespaced=matches[0]
    old=function(text,namespaced)
    # PORT: Missing upstream rodata is read from the owned overlay at runtime;
    # no extracted bytes, guesses, or generated disassembly enter the repository.
    code=code.replace('    s32    i;', '    (void)arg0;\n    audio_map0_tables();\n    s32    i;')
    code=code.replace('    u32    saveByte;', '').replace('    saveByte = g_SavegamePtr->eventFlags[0];','')
    code=code.replace('&D_800DF2F8','&audio_map0_limits').replace('D_800DF300','audio_map0_flags')
    text=text.replace(old,code.replace(name,namespaced))
    text=text.replace('#include "npc_startup.h"','#include "npc_startup.h"\n#include "audio_records.h"')
    mapfile.write_text(text,encoding='utf-8')
    test=out/'render_milestone.rs'
    text=test.read_text(encoding='utf-8')
    text=text.replace('port_capture_render_boundary','port_audio_capture_render_boundary')
    text=text.replace('(2266, 11, 0)','(2274, 11, 0)').replace('(154, 2)','(148, 2)')
    text=text.replace('work/world','work/audio')
    text=text.replace('original BGM layer-controller guard','next original player-collision guard after BGM')
    test.write_text(text,encoding='utf-8')


def milestones(root):
    """Retain existing checkpoint assertions with the measured bank-load delay."""
    import array
    import json
    import os
    import subprocess
    import wave
    from datetime import datetime, timezone
    out=root.parent.parent/'private/work/audio/milestones'/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out.mkdir(parents=True)
    replay_root=root/'docs/core/replays'
    manifest=json.loads((replay_root/'milestones.json').read_text())
    flags={'state':'--expect-state','step':'--expect-step','menu':'--expect-menu','option_entry':'--expect-option-entry','movie_skips':'--expect-movie-skips','min_movie_frames':'--min-movie-frames','min_lit_pixels':'--min-lit-pixels'}
    results=[]
    env=dict(os.environ)
    env.pop('SH_AUDIO_AMBIENCE_PROBE',None)
    for case in manifest['verified']:
        # Original BASE/FIRST loading adds 24 VBlanks before movie startup.
        # Keep every input's timing relative to the measured startup and retain
        # all original state, movie-frame, skip and visible-pixel assertions.
        replay=out/(case['name']+'.txt')
        rows=[]
        for line in (replay_root/case['input']).read_text().splitlines():
            clean=line.split('#')[0].strip()
            if clean:
                tick,bits=clean.split();rows.append(f'{int(tick)+(24 if int(tick) else 0)} {bits}')
        replay.write_text('\n'.join(rows)+'\n')
        screenshot=out/(case['name']+'.png')
        wav=out/(case['name']+'.wav')
        record=case['name'] in ['movie-end-title','new-game-menu']
        command=[str(root/'target/release/silent-hill-boot.exe'),'--headless','--audio','wav:'+str(wav) if record else 'off','--frames',str(case['frames']+24),'--input',str(replay),'--screenshot',str(screenshot)]
        for key,flag in flags.items():
            if key in case:command.extend([flag,str(case[key])])
        run=subprocess.run(command,cwd=root,env=env,capture_output=True,text=True,timeout=180)
        log=run.stdout+run.stderr
        (out/(case['name']+'.log')).write_text(log,encoding='utf-8')
        result={'name':case['name'],'pass':run.returncode==0,'exit':run.returncode,'checkpoint':[l for l in log.splitlines() if l.startswith('CHECK ')]}
        if record:
            with wave.open(str(wav)) as source:
                samples=array.array('h',source.readframes(source.getnframes()))
                if case['name']=='new-game-menu':samples=samples[(1510+24)*735*2:(1650+24)*735*2]
            result.update(peak=max(map(abs,samples),default=0),nonzero=sum(x!=0 for x in samples),rails=sum(x in [-32768,32767] for x in samples))
            result['pass'] &= result['nonzero']>0 and result['rails']==0
        results.append(result)
        print(('PASS ' if result['pass'] else 'FAIL ')+json.dumps(result),flush=True)
        if not result['pass']:break
    # The map audio continuation is explicit and does not hide the player guard.
    wav=out/'first-map-ambience.wav'
    run=subprocess.run([str(root/'target/release/silent-hill-boot.exe'),'--headless','--audio','wav:'+str(wav),'--frames','3720','--input',str(replay_root/'first_map.txt')],cwd=root,env=dict(env,SH_AUDIO_AMBIENCE_PROBE='1'),capture_output=True,text=True,timeout=180)
    log=run.stdout+run.stderr
    (out/'first-map-ambience.log').write_text(log,encoding='utf-8')
    with wave.open(str(wav)) as source:
        samples=array.array('h',source.readframes(source.getnframes()))[2274*735*2:]
    result={'name':'first-map-ambience','exit':run.returncode,'frames':len(samples)//2,'peak':max(map(abs,samples),default=0),'nonzero':sum(x!=0 for x in samples),'rails':sum(x in [-32768,32767] for x in samples)}
    result['pass']=run.returncode==1 and 'World_NearbyPlayerCollisionTriggersGet/native nearby trigger classification at state=11 step=2 VBlank=2274' in log and 'AUDIO_AMBIENCE_PROBE end midi_task=770' in log and 'AUDIO_LAYER_STOP_PROBE PASS raw_task=65535' in log and result['frames']==300*735 and result['nonzero']>0 and result['rails']==0
    results.append(result)
    print(('PASS ' if result['pass'] else 'FAIL ')+json.dumps(result),flush=True)
    (out/'results.json').write_text(json.dumps(results,indent=2))
    print('Evidence: '+str(out),flush=True)
    return 0 if len(results)==11 and all(r['pass'] for r in results) else 1


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--decomp',default=Path('game/decomp'),type=Path)
    parser.add_argument('--out',type=Path)
    parser.add_argument('--check-clang',type=Path)
    parser.add_argument('--milestones',action='store_true')
    args=parser.parse_args()
    if args.milestones: raise SystemExit(milestones(Path(__file__).resolve().parents[1]))
    if args.check_clang:
        import subprocess
        from prepare_sdk import prepare as prepare_sdk
        root=Path(__file__).resolve().parents[1]
        out=args.out or root/'target/audio-clang'
        prepare_sdk(args.decomp,out)
        prepare(args.decomp,out)
        sources=[out/'audio_driver.c',root/'port/audio_services.c',root/'port/audio_session.c',out/'map0_s00.c']
        sources += [out/('audio_caller_'+n) for n in ['runtime.c','gameplay.c','npc_startup.c','title_services.c']]
        flags=['-target','aarch64-apple-ios15.0','-ffreestanding','-std=c11','-Wall','-Wextra','-Werror','-nostdinc']
        for inc in [root/'tools/layout-include',root/'port',root/'port/include',out,args.decomp/'include',args.decomp/'src/main']:
            flags += ['-I',str(inc.resolve())]
        for name in ['ccos','csin','csqrt','catan']: flags.append('-fno-builtin-'+name)
        failed=0
        for source in sources:
            run=subprocess.run([str(args.check_clang),str(source),'--checks=-*,clang-analyzer-core.DivideZero','--warnings-as-errors=*','--',*flags],capture_output=True,text=True)
            print(('FAIL' if run.returncode else 'PASS')+': '+source.name,flush=True)
            if run.returncode: failed+=1;print(run.stdout+run.stderr,flush=True)
        print(f'Audio arm64: {len(sources)-failed} passed, {failed} failed; Apple SDK/device untested.')
        raise SystemExit(bool(failed))
    if not args.out: parser.error('--out is required when generating')
    prepare(args.decomp,args.out)
