/***************************************************************************
 *   PCSX-Revolution - PlayStation Emulator for Nintendo Wii               *
 *   Copyright (C) 2009-2010  PCSX-Revolution Dev Team                     *
 *   <http://code.google.com/p/pcsx-revolution/>                           *
 *                                                                         *
 *   This program is free software; you can redistribute it and/or modify  *
 *   it under the terms of the GNU General Public License as published by  *
 *   the Free Software Foundation; either version 2 of the License, or     *
 *   (at your option) any later version.                                   *
 *                                                                         *
 *   This program is distributed in the hope that it will be useful,       *
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of        *
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
 *   GNU General Public License for more details.                          *
 *                                                                         *
 *   You should have received a copy of the GNU General Public License     *
 *   along with this program; if not, write to the                         *
 *   Free Software Foundation, Inc.,                                       *
 *   51 Franklin Street, Fifth Floor, Boston, MA 02111-1307 USA.           *
 ***************************************************************************/

/*
* Standalone adaptation of PCSX-ReARMed 9d456e2c85df6a4d1f1cfd0ec8a5e81ed9bff42a.
* GTE functions.
*/


#include "portable.h"
#include "psxgpu_gte.h"

#ifndef GTE_LOG
#define GTE_LOG(...)
#endif

#ifdef FLAGLESS
#define NF _nf
#define NM(n) n##_nf
#else
#define NF
#define NM(n) n
#endif

#define VX(n) (n < 3 ? regs->CP2D.p[n << 1].sw.l : regs->CP2D.p[9].sw.l)
#define VY(n) (n < 3 ? regs->CP2D.p[n << 1].sw.h : regs->CP2D.p[10].sw.l)
#define VZ(n) (n < 3 ? regs->CP2D.p[(n << 1) + 1].sw.l : regs->CP2D.p[11].sw.l)

#define fSX(n) ((regs->CP2D.p)[((n) + 12)].sw.l)
#define fSY(n) ((regs->CP2D.p)[((n) + 12)].sw.h)
#define fSZ(n) ((regs->CP2D.p)[((n) + 17)].w.l) /* (n == 0) => SZ1; */

#define fR(x)    (regs->CP2D.p[(x) + 20].b.l)
#define fG(x)    (regs->CP2D.p[(x) + 20].b.h)
#define fB(x)    (regs->CP2D.p[(x) + 20].b.h2)
#define fCODE(x) (regs->CP2D.p[(x) + 20].b.h3)

#define gteVXY0 (regs->CP2D.r[0])
#define gteVX0  (regs->CP2D.p[0].sw.l)
#define gteVY0  (regs->CP2D.p[0].sw.h)
#define gteVZ0  (regs->CP2D.p[1].sw.l)
#define gteVXY1 (regs->CP2D.r[2])
#define gteVX1  (regs->CP2D.p[2].sw.l)
#define gteVY1  (regs->CP2D.p[2].sw.h)
#define gteVZ1  (regs->CP2D.p[3].sw.l)
#define gteVXY2 (regs->CP2D.r[4])
#define gteVX2  (regs->CP2D.p[4].sw.l)
#define gteVY2  (regs->CP2D.p[4].sw.h)
#define gteVZ2  (regs->CP2D.p[5].sw.l)
#define gteRGB  (regs->CP2D.r[6])
#define gteR    (regs->CP2D.p[6].b.l)
#define gteG    (regs->CP2D.p[6].b.h)
#define gteB    (regs->CP2D.p[6].b.h2)
#define gteCODE (regs->CP2D.p[6].b.h3)
#define gteOTZ  (regs->CP2D.p[7].w.l)
#define gteIR0  (regs->CP2D.p[8].sw.l)
#define gteIR1  (regs->CP2D.p[9].sw.l)
#define gteIR2  (regs->CP2D.p[10].sw.l)
#define gteIR3  (regs->CP2D.p[11].sw.l)
#define gteSXY0 (regs->CP2D.r[12])
#define gteSX0  (regs->CP2D.p[12].sw.l)
#define gteSY0  (regs->CP2D.p[12].sw.h)
#define gteSXY1 (regs->CP2D.r[13])
#define gteSX1  (regs->CP2D.p[13].sw.l)
#define gteSY1  (regs->CP2D.p[13].sw.h)
#define gteSXY2 (regs->CP2D.r[14])
#define gteSX2  (regs->CP2D.p[14].sw.l)
#define gteSY2  (regs->CP2D.p[14].sw.h)
#define gteSXYP (regs->CP2D.r[15])
#define gteSXP  (regs->CP2D.p[15].sw.l)
#define gteSYP  (regs->CP2D.p[15].sw.h)
#define gteSZ0  (regs->CP2D.p[16].w.l)
#define gteSZ1  (regs->CP2D.p[17].w.l)
#define gteSZ2  (regs->CP2D.p[18].w.l)
#define gteSZ3  (regs->CP2D.p[19].w.l)
#define gteRGB0  (regs->CP2D.r[20])
#define gteR0    (regs->CP2D.p[20].b.l)
#define gteG0    (regs->CP2D.p[20].b.h)
#define gteB0    (regs->CP2D.p[20].b.h2)
#define gteCODE0 (regs->CP2D.p[20].b.h3)
#define gteRGB1  (regs->CP2D.r[21])
#define gteR1    (regs->CP2D.p[21].b.l)
#define gteG1    (regs->CP2D.p[21].b.h)
#define gteB1    (regs->CP2D.p[21].b.h2)
#define gteCODE1 (regs->CP2D.p[21].b.h3)
#define gteRGB2  (regs->CP2D.r[22])
#define gteR2    (regs->CP2D.p[22].b.l)
#define gteG2    (regs->CP2D.p[22].b.h)
#define gteB2    (regs->CP2D.p[22].b.h2)
#define gteCODE2 (regs->CP2D.p[22].b.h3)
#define gteRES1  (regs->CP2D.r[23])
#define gteMAC0  (((s32 *)regs->CP2D.r)[24])
#define gteMAC1  (((s32 *)regs->CP2D.r)[25])
#define gteMAC2  (((s32 *)regs->CP2D.r)[26])
#define gteMAC3  (((s32 *)regs->CP2D.r)[27])
#define gteIRGB  (regs->CP2D.r[28])
#define gteORGB  (regs->CP2D.r[29])
#define gteLZCS  (regs->CP2D.r[30])
#define gteLZCR  (regs->CP2D.r[31])

#define gteR11R12 (((s32 *)regs->CP2C.r)[0])
#define gteR22R23 (((s32 *)regs->CP2C.r)[2])
#define gteR11 (regs->CP2C.p[0].sw.l)
#define gteR12 (regs->CP2C.p[0].sw.h)
#define gteR13 (regs->CP2C.p[1].sw.l)
#define gteR21 (regs->CP2C.p[1].sw.h)
#define gteR22 (regs->CP2C.p[2].sw.l)
#define gteR23 (regs->CP2C.p[2].sw.h)
#define gteR31 (regs->CP2C.p[3].sw.l)
#define gteR32 (regs->CP2C.p[3].sw.h)
#define gteR33 (regs->CP2C.p[4].sw.l)
#define gteTRX (((s32 *)regs->CP2C.r)[5])
#define gteTRY (((s32 *)regs->CP2C.r)[6])
#define gteTRZ (((s32 *)regs->CP2C.r)[7])
#define gteL11 (regs->CP2C.p[8].sw.l)
#define gteL12 (regs->CP2C.p[8].sw.h)
#define gteL13 (regs->CP2C.p[9].sw.l)
#define gteL21 (regs->CP2C.p[9].sw.h)
#define gteL22 (regs->CP2C.p[10].sw.l)
#define gteL23 (regs->CP2C.p[10].sw.h)
#define gteL31 (regs->CP2C.p[11].sw.l)
#define gteL32 (regs->CP2C.p[11].sw.h)
#define gteL33 (regs->CP2C.p[12].sw.l)
#define gteRBK (((s32 *)regs->CP2C.r)[13])
#define gteGBK (((s32 *)regs->CP2C.r)[14])
#define gteBBK (((s32 *)regs->CP2C.r)[15])
#define gteLR1 (regs->CP2C.p[16].sw.l)
#define gteLR2 (regs->CP2C.p[16].sw.h)
#define gteLR3 (regs->CP2C.p[17].sw.l)
#define gteLG1 (regs->CP2C.p[17].sw.h)
#define gteLG2 (regs->CP2C.p[18].sw.l)
#define gteLG3 (regs->CP2C.p[18].sw.h)
#define gteLB1 (regs->CP2C.p[19].sw.l)
#define gteLB2 (regs->CP2C.p[19].sw.h)
#define gteLB3 (regs->CP2C.p[20].sw.l)
#define gteRFC (((s32 *)regs->CP2C.r)[21])
#define gteGFC (((s32 *)regs->CP2C.r)[22])
#define gteBFC (((s32 *)regs->CP2C.r)[23])
#define gteOFX (((s32 *)regs->CP2C.r)[24])
#define gteOFY (((s32 *)regs->CP2C.r)[25])
// senquack - gteH register is u16, not s16, and used in GTE that way.
//  HOWEVER when read back by CPU using CFC2, it will be incorrectly
//  sign-extended by bug in original hardware, according to Nocash docs
//  GTE section 'Screen Offset and Distance'. The emulator does this
//  sign extension when it is loaded to GTE by CTC2.
//#define gteH   (regs->CP2C.p[26].sw.l)
#define gteH   (regs->CP2C.p[26].w.l)
#define gteDQA (regs->CP2C.p[27].sw.l)
#define gteDQB (((s32 *)regs->CP2C.r)[28])
#define gteZSF3 (regs->CP2C.p[29].sw.l)
#define gteZSF4 (regs->CP2C.p[30].sw.l)
#define gteFLAG (regs->CP2C.r[31])

#define GTE_SF(op) ((op >> 19) & 1)
#define GTE_MX(op) ((op >> 17) & 3)
#define GTE_V(op) ((op >> 15) & 3)
#define GTE_CV(op) ((op >> 13) & 3)
#define GTE_CD(op) ((op >> 11) & 3) /* not used */
#define GTE_LM(op) ((op >> 10) & 1)
#define GTE_CT(op) ((op >> 6) & 15) /* not used */

// shift the gte 44bit accumulator to 64bit
#define MAC123_SHIFT (32-12)

// mac 123 without flags (expensive to calculate, rarely used)
// if your platform is slow, consider adding it here
#if !defined(GTE_MAC123_FLAGLESS) && (defined(FLAGLESS) || (defined(PSXGPU_UNUSED_ARM_ASM) && !defined(HAVE_ARMV5)))
#define GTE_MAC123_FLAGLESS 1
#endif

#if GTE_MAC123_FLAGLESS

static inline s64 mac123add4(u32 id, u32 *flags, s32 a1, s32 a2, s32 a3, s32 a4, int shift) {
	return (((s64)a1 << 12) + a2 + a3 + a4) >> shift;
}

static inline s32 mac123add_s12(u32 id, u32 *flags, s32 in12, s32 addend, int shift) {
	return (((s64)in12 << 12) + addend) >> shift;
}

static inline s32 mac123sub_s12(u32 id, u32 *flags, s32 in12, s32 subtrahend, int shift) {
	return (((s64)in12 << 12) - subtrahend) >> shift;
}

#else

static inline s64 mac123add(u32 id, u32 *flags, s64 in, s32 addend) {
	s64 a;
#if defined(PSXGPU_UNUSED_ARM_ASM)
	u32 flag = 1u << (31 - id);
	asm("adds %Q[a], %Q[in], %[add], lsl #20\n"
	    "adcs %R[a], %R[in], %[add], asr #12\n"
	    "movpl %[flag], %[flag], lsr #3\n"
	    "orrvs %[flags], %[flags], %[flag]"
	    : [a]"=&r"(a), [flags]"+&r"(*flags), [flag]"+&r"(flag)
	    : [in]"r"(in), [add]"r"(addend)
	    : "cc");
#elif 0 // defined(PSXGPU_UNUSED_ARM64_ASM) // slower
	u32 flag = 1u << (31 - id);
	u32 flagpl = 1u << (31 - id - 3);
	s64 add_ = addend;
	asm("adds %[a], %[in], %[add], lsl %[shift]\n"
	    "csel %w[flag], %w[flagpl], %w[flag], pl\n"
	    "csel %w[flag], %w[flag], wzr, vs\n"
	    : [a]"=&r"(a), [flag]"+&r"(flag)
	    : [in]"r"(in), [add]"r"(add_), [flagpl]"r"(flagpl), [shift]"i"(MAC123_SHIFT)
	    : "cc");
	*flags |= flag;
#else
	int o = add_overflow(in, (s64)addend * (1LL << MAC123_SHIFT), &a);
	*flags |= (o && a <  0) << (31 - id);
	*flags |= (o && a >= 0) << (28 - id);
#endif
	return a;
}

static inline s64 mac123add4(u32 id, u32 *flags, s32 a1, s32 a2, s32 a3, s32 a4, int shift) {
	s64 a = (s64)((u64)(u32)a1 << (12 + MAC123_SHIFT));
	a = mac123add(id, flags, a, a2);
	a = mac123add(id, flags, a, a3);
	a = mac123add(id, flags, a, a4);
	return a >> (shift + MAC123_SHIFT);
}

static inline s32 mac123add_s12(u32 id, u32 *flags, s32 in12, s32 addend, int shift) {
	return (s32)(mac123add(id, flags, (s64)((u64)(u32)in12 << (12+MAC123_SHIFT)), addend) >> (shift+MAC123_SHIFT));
}

static inline s64 mac123sub_s12(u32 id, u32 *flags, s32 in12, s32 subtrahend, int shift) {
	s64 a;
#if defined(PSXGPU_UNUSED_ARM_ASM)
	u32 flag = 1u << (31 - id);
	s64 in = (s64)((u64)(u32)in12 << (12+MAC123_SHIFT));
	asm("subs %Q[a], %Q[in], %[sub], lsl #20\n"
	    "sbcs %R[a], %R[in], %[sub], asr #12\n"
	    "movpl %[flag], %[flag], lsr #3\n"
	    "orrvs %[flags], %[flags], %[flag]"
	    : [a]"=&r"(a), [flags]"+&r"(*flags), [flag]"+&r"(flag)
	    : [in]"r"(in), [sub]"r"(subtrahend)
	    : "cc");
#elif 0 // defined(PSXGPU_UNUSED_ARM64_ASM)
	u32 flag = 1u << (31 - id);
	u32 flagpl = 1u << (31 - id - 3);
	s64 in = (s64)((u64)(u32)in12 << (12+MAC123_SHIFT));
	s64 sub_ = subtrahend;
	asm("subs %[a], %[in], %[sub], lsl %[shift]\n"
	    "csel %w[flag], %w[flagpl], %w[flag], pl\n"
	    "csel %w[flag], %w[flag], wzr, vs\n"
	    : [a]"=&r"(a), [flag]"+&r"(flag)
	    : [in]"r"(in), [sub]"r"(sub_), [flagpl]"r"(flagpl), [shift]"i"(MAC123_SHIFT)
	    : "cc");
	*flags |= flag;
#else
	int o = sub_overflow((s64)((u64)(u32)in12 << (12+MAC123_SHIFT)), (s64)subtrahend * (1LL << MAC123_SHIFT), &a);
	*flags |= (o && subtrahend <  0) << (31 - id);
	*flags |= (o && subtrahend >= 0) << (28 - id);
#endif
	return a >> (shift+MAC123_SHIFT);
}

#endif // !FLAGLESS for mac 123

#ifndef FLAGLESS

static inline s64 mac0flags(u32 *flags, s64 a) {
#if 1
	if (a != (s32)a)
		*flags |= 1u << (16 + (a >> 63));
#else
	if (a > 0x7fffffff)
		*flags |= 1u << 16;
	if (a < -(s64)0x80000000)
		*flags |= 1u << 15;
#endif
	return a;
}

#if defined(PSXGPU_UNUSED_ARM_ASM)

#define LIM(flags_, value_, max_, min_, flag_) \
({s32 r_ = value_; \
  asm("cmp   %[val], %[max]\n" \
      "movgt %[val], %[max]\n" \
      "orrgt %[flags], %[flag]\n" \
      "cmp   %[val], %[min]\n" \
      "movlt %[val], %[min]\n" \
      "orrlt %[flags], %[flag]\n" \
      : [val]"+&r"(r_), [flags]"+&r"(*(flags_)) \
      : [max]"r"(max_), [min]"r"(min_), [flag]"i"(flag_) \
      : "cc"); \
  r_;})

#elif defined(PSXGPU_UNUSED_ARM64_ASM)

#define LIM(flags_, value_, max_, min_, flagc_) \
({s32 r_ = value_; \
  u32 flag_o_, flag_ = flagc_; \
  asm("cmp  %w[val], %w[max]\n" \
      "csel %w[val], %w[max], %w[val], gt\n" \
      "csel %w[flag_o], %w[flag], wzr, gt\n" \
      "cmp  %w[val], %w[min]\n" \
      "csel %w[val], %w[min], %w[val], lt\n" \
      "csel %w[flag_o], %w[flag], %w[flag_o], lt\n" \
      : [val]"+&r"(r_), [flag_o]"=&r"(flag_o_) \
      : [max]"r"(max_), [min]"r"(min_), [flag]"r"(flag_) \
      : "cc"); \
  *(flags_) |= flag_o_; \
  r_;})

#else

static inline s32 LIM(u32 *flags, s32 value, s32 max, s32 min, u32 flag) {
	s32 ret = value;
	if (ret > max)
		ret = max;
	if (ret < min)
		ret = min;
	if (ret != value)
		*flags |= flag;
	return ret;
}

#endif

static inline void LIMF(u32 *flags, s32 value, s32 max, s32 min, u32 flag) {
	if (value > max || value < min)
		*flags |= flag;
}

static inline u32 getFinalFlag(u32 flags) {
	flags |= ~((flags & 0x7f87e000u) - 1) & (1u << 31);
	return flags;
}

#else

static inline s64 mac0flags(u32 *flags, s64 a) {
	return a;
}

static inline s32 LIM(u32 *flags, s32 value, s32 max, s32 min, u32 flag) {
	s32 ret = value;
	if (ret > max)
		ret = max;
	if (ret < min)
		ret = min;
	return ret;
}

#define LIMF(flags, (s32)(a), ...) (void)(a)

static inline u32 getFinalFlag(u32 flags) {
	return 0;
}

#endif

#define limB1(flags, a, l)   LIM(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 24))
#define limB2(flags, a, l)   LIM(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 23))
#define limB3(flags, a, l)   LIM(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 22))
#define limBF1(flags, a, l) LIMF(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 24))
#define limBF2(flags, a, l) LIMF(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 23))
#define limBF3(flags, a, l) LIMF(flags, (s32)(a), 0x7fff, -0x8000 * !l, (1u << 22))
#define limC1(flags, a) LIM(flags, (s32)(a), 0x00ff, 0x0000, (1u << 21))
#define limC2(flags, a) LIM(flags, (s32)(a), 0x00ff, 0x0000, (1u << 20))
#define limC3(flags, a) LIM(flags, (s32)(a), 0x00ff, 0x0000, (1u << 19))
#define limD(flags, a)  LIM(flags, (s32)(a), 0xffff, 0x0000, (1u << 18))
#define limG1(flags, a) LIM(flags, (s32)(a),  0x3ff, -0x400, (1u << 14))
#define limG2(flags, a) LIM(flags, (s32)(a),  0x3ff, -0x400, (1u << 13))
#define limH(flags, a)  LIM(flags, (s32)(a), 0x1000, 0x0000, (1u << 12))

#ifndef FLAGLESS


static inline u32 ir2rgb(s32 ir)
{
	ir >>= 7;
	if (ir < 0)
		ir = 0;
	else if (ir > 0x1f)
		ir = 0x1f;
	return ir;
}

static u32 MFC2(struct psxCP2Regs *regs, int reg) {
	switch (reg) {
		case 1:
		case 3:
		case 5:
		case 8:
		case 9:
		case 10:
		case 11:
			regs->CP2D.r[reg] = (s32)regs->CP2D.p[reg].sw.l;
			break;

		case 7:
		case 16:
		case 17:
		case 18:
		case 19:
			regs->CP2D.r[reg] = (u32)regs->CP2D.p[reg].w.l;
			break;

		case 15:
			regs->CP2D.r[reg] = gteSXY2;
			break;

		case 28:
		case 29:
			regs->CP2D.r[reg] = ir2rgb(gteIR1) | (ir2rgb(gteIR2) << 5) | (ir2rgb(gteIR3) << 10);
			break;
	}
	return regs->CP2D.r[reg];
}

static u32 lzc(s32 val) { return clz32((u32)(val ^ (val >> 31))); }

static void MTC2(struct psxCP2Regs *regs, u32 value, int reg) {
	switch (reg) {
		case 15:
			gteSXY0 = gteSXY1;
			gteSXY1 = gteSXY2;
			gteSXY2 = value;
			gteSXYP = value;
			break;

		case 28:
			gteIRGB = value;
			// not gteIR1 etc. just to be consistent with dynarec
			regs->CP2D.r[9] = (value & 0x1f) << 7;
			regs->CP2D.r[10] = (value & 0x3e0) << 2;
			regs->CP2D.r[11] = (value & 0x7c00) >> 3;
			break;

		case 30:
			gteLZCS = value;
			gteLZCR = lzc(value);
			break;

		case 31:
			return;

		default:
			regs->CP2D.r[reg] = value;
	}
}

static void CTC2(struct psxCP2Regs *regs, u32 value, int reg) {
	switch (reg) {
		case 4:
		case 12:
		case 20:
		case 26:
		case 27:
		case 29:
		case 30:
			value = (s32)(s16)value;
			break;

		case 31:
			value = getFinalFlag(value & 0x7ffff000);
			break;
	}

	regs->CP2C.r[reg] = value;
}

#endif // FLAGLESS

//#define GTE_USE_NATIVE_DIVIDE
#ifdef GTE_USE_NATIVE_DIVIDE

// more mathematically correct, but not what the real hardware does
// (wrong for ~24.72% denominators)
static inline u32 DIVIDE(u16 n, u16 d) {
	return (((u32)n << 16) + (d >> 1)) / d;
}

#else
#include "gte_divider.h"
#endif

static inline s32 divide(u32 *flags, u16 h, u16 sz3)
{
	if (likely(h < sz3 * 2u)) {
		s32 r = DIVIDE(h, sz3);
		return r >= 0x1ffff ? 0x1ffff : r;
	}
#ifndef FLAGLESS
	*flags |= 1u << 17;
#endif
	return 0x1ffff;
}

#ifdef HAVE_ARMV5

#define gteMAC123f gteMAC123f_arm

#else

static u32 gteMAC123f(psxCP2Regs *regs, s32 vx, s32 vy, s32 vz,
	const s16 *mx, const s32 *cv, int shift)
{
	u32 flags = 0;

	gteMAC1 = (s32)(mac123add4(1, &flags, cv[0], mx[0] * vx, mx[1] * vy, mx[2] * vz, shift));
	gteMAC2 = (s32)(mac123add4(2, &flags, cv[1], mx[3] * vx, mx[4] * vy, mx[5] * vz, shift));
	gteMAC3 = (s32)(mac123add4(3, &flags, cv[2], mx[6] * vx, mx[7] * vy, mx[8] * vz, shift));

	return flags;
}

#endif

static inline force_inline void gteRTPS(psxCP2Regs *regs, int shift, int lm)
{
	s32 vx = gteVX0, vy = gteVY0, vz = gteVZ0;
	s32 sz3, quotient;
	s32 mac1, mac2;
	s64 mac3, mac0;
	u32 flags = 0;

	GTE_LOG("GTE RTPS\n");

	gteMAC1 = (s32)(mac1 = (s32)(mac123add4(1, &flags, gteTRX, gteR11 * vx, gteR12 * vy, gteR13 * vz, shift)));
	gteMAC2 = (s32)(mac2 = (s32)(mac123add4(2, &flags, gteTRY, gteR21 * vx, gteR22 * vy, gteR23 * vz, shift)));
	gteMAC3 = (s32)((s32)(mac3 = mac123add4(3, &flags, gteTRZ, gteR31 * vx, gteR32 * vy, gteR33 * vz, shift)));
	gteIR1 = (s16)(limB1(&flags, mac1, lm));
	gteIR2 = (s16)(limB2(&flags, mac2, lm));
	gteIR3 = (s16)(LIM(&flags, (s32)mac3, 0x7fff, -0x8000 * !lm, 0));
	sz3 = (s32)(mac3 >> (12-shift));
	limBF3(&flags, sz3, 0);
	sz3 = limD(&flags, sz3);
	quotient = divide(&flags, gteH, (u16)sz3);
	gteSZ0 = (u16)(gteSZ1);
	gteSZ1 = (u16)(gteSZ2);
	gteSZ2 = (u16)(gteSZ3);
	gteSZ3 = (u16)(sz3);
	gteSXY0 = gteSXY1;
	gteSXY1 = gteSXY2;

	gteSX2 = (s16)(limG1(&flags, (s32)(mac0flags(&flags, gteOFX + (s64)gteIR1 * quotient) >> 16)));
	gteSY2 = (s16)(limG2(&flags, (s32)(mac0flags(&flags, gteOFY + (s64)gteIR2 * quotient) >> 16)));

	gteMAC0 = (s32)(mac0 = mac0flags(&flags, gteDQB + (s64)gteDQA * quotient));
	gteIR0 = (s16)(limH(&flags, mac0 >> 12));
	gteFLAG = getFinalFlag(flags);
}

static inline force_inline void gteRTPT(psxCP2Regs *regs, int shift, int lm)
{
	s32 sz3, quotient;
	s32 mac1, mac2;
	s64 mac3, mac0;
	s32 vx, vy, vz;
	s32 ir1, ir2;
	u32 h = gteH;
	u32 flags = 0;
	int v;

	GTE_LOG("GTE RTPT\n");

	gteSZ0 = (u16)(gteSZ3);
	for (v = 0; v < 3; v++) {
		vx = VX(v);
		vy = VY(v);
		vz = VZ(v);
		mac1 = (s32)(mac123add4(1, &flags, gteTRX, gteR11 * vx, gteR12 * vy, gteR13 * vz, shift));
		mac2 = (s32)(mac123add4(2, &flags, gteTRY, gteR21 * vx, gteR22 * vy, gteR23 * vz, shift));
		mac3 = mac123add4(3, &flags, gteTRZ, gteR31 * vx, gteR32 * vy, gteR33 * vz, shift);
		ir1 = limB1(&flags, mac1, lm);
		ir2 = limB2(&flags, mac2, lm);
		sz3 = (s32)(mac3 >> (12-shift));
		limBF3(&flags, sz3, 0);
		sz3 = limD(&flags, sz3);
		quotient = divide(&flags, (u16)h, (u16)sz3);
		fSZ(v) = (u16)(sz3);
		fSX(v) = (s16)(limG1(&flags, mac0flags(&flags, gteOFX + (s64)ir1 * quotient) >> 16));
		fSY(v) = (s16)(limG2(&flags, mac0flags(&flags, gteOFY + (s64)ir2 * quotient) >> 16));
	}

	gteMAC1 = (s32)(mac1);
	gteMAC2 = (s32)(mac2);
	gteMAC3 = (s32)(mac3);
	gteIR1 = (s16)(ir1);
	gteIR2 = (s16)(ir2);
	gteIR3 = (s16)(LIM(&flags, (s32)mac3, 0x7fff, -0x8000 * !lm, 0));
	gteMAC0 = (s32)(mac0 = mac0flags(&flags, gteDQB + (s64)gteDQA * quotient));
	gteIR0 = (s16)(limH(&flags, mac0 >> 12));
	gteFLAG = getFinalFlag(flags);
}

static inline force_inline void NM(gteMVMVAn)(psxCP2Regs *regs,
	const s16 *mx, const s16 *v, const s32 *cv, int shift, int lm)
{
	u32 flags = 0;

	flags = gteMAC123f(regs, v[0], v[1], v[2], mx, cv, shift);
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteFLAG = getFinalFlag(flags);
}

static noinline void NM(gteMVMVAbugged)(psxCP2Regs *regs,
	const s16 *mx, const s16 *v, const s32 *cv, int shift, int lm)
{
	s32 vx = v[0], vy = v[1], vz = v[2];
	u32 flags = 0;
	s32 mac1 = mac123add_s12(1, &flags, cv[0], mx[0] * vx, shift);
	s32 mac2 = mac123add_s12(2, &flags, cv[1], mx[3] * vx, shift);
	s32 mac3 = mac123add_s12(3, &flags, cv[2], mx[6] * vx, shift);
	limBF1(&flags, mac1, 0);
	limBF2(&flags, mac2, 0);
	limBF3(&flags, mac3, 0);
	gteMAC1 = (s32)(((s64)(mx[1] * vy) + (mx[2] * vz)) >> shift);
	gteMAC2 = (s32)(((s64)(mx[4] * vy) + (mx[5] * vz)) >> shift);
	gteMAC3 = (s32)(((s64)(mx[7] * vy) + (mx[8] * vz)) >> shift);
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteFLAG = getFinalFlag(flags);
}

static noinline void NM(gteMVMVAn_sf0lm0)(psxCP2Regs *regs,
			const s16 *mx, const s16 *v, const s32 *cv) {
	NM(gteMVMVAn)(regs, mx, v, cv,  0, 0);
}
static noinline void NM(gteMVMVAn_sf0lm1)(psxCP2Regs *regs,
			const s16 *mx, const s16 *v, const s32 *cv) {
	NM(gteMVMVAn)(regs, mx, v, cv,  0, 1);
}
static noinline void NM(gteMVMVAn_sf1lm0)(psxCP2Regs *regs,
			const s16 *mx, const s16 *v, const s32 *cv) {
	NM(gteMVMVAn)(regs, mx, v, cv, 12, 0);
}
static noinline void NM(gteMVMVAn_sf1lm1)(psxCP2Regs *regs,
			const s16 *mx, const s16 *v, const s32 *cv) {
	NM(gteMVMVAn)(regs, mx, v, cv, 12, 1);
}

static inline force_inline void gteMVMVA(psxCP2Regs *regs,
	int mx, int v, int cv, int shift, int lm)
{
	const s16 v3[3] = { regs->CP2D.p[9].sw.l, regs->CP2D.p[10].sw.l, regs->CP2D.p[11].sw.l };
	const s16 *vp = &regs->CP2D.p[v << 1].sw.l;
	const s16 *mxp = &regs->CP2C.p[mx << 3].sw.l;
	const s32 *cvp = (s32 *)&regs->CP2C.r[(cv << 3) + 5];
	const s32 cv3[3] = { 0, 0, 0 };
	s16 mx3[9];

	GTE_LOG("GTE MVMVA\n");

	if (v == 3)
		vp = v3;
	if (unlikely(mx == 3)) {
		mxp = mx3;
		mx3[0] = -(regs->CP2D.p[6].b.l * 16);
		mx3[1] =  regs->CP2D.p[6].b.l << 4;
		mx3[2] = regs->CP2D.p[8].sw.l;
		mx3[3] = regs->CP2C.p[1].sw.l;
		mx3[4] = regs->CP2C.p[1].sw.l;
		mx3[5] = regs->CP2C.p[1].sw.l;
		mx3[6] = regs->CP2C.p[2].sw.l;
		mx3[7] = regs->CP2C.p[2].sw.l;
		mx3[8] = regs->CP2C.p[2].sw.l;
	}
	if (unlikely(cv == 3))
		cvp = cv3;
	if (unlikely(cv == 2))
		NM(gteMVMVAbugged)(regs, mxp, vp, cvp, shift, lm);
	else {
		if (shift && lm)
			NM(gteMVMVAn_sf1lm1)(regs, mxp, vp, cvp);
		else if (shift && !lm)
			NM(gteMVMVAn_sf1lm0)(regs, mxp, vp, cvp);
		else if (lm)
			NM(gteMVMVAn_sf0lm1)(regs, mxp, vp, cvp);
		else
			NM(gteMVMVAn_sf0lm0)(regs, mxp, vp, cvp);
	}
}

static void NM(gteMVMVA_generic)(psxCP2Regs *regs, u32 code)
{
	gteMVMVA(regs, GTE_MX(code), GTE_V(code), GTE_CV(code),
	         12 * GTE_SF(code), GTE_LM(code));
}

static inline void gteNCLIP_(psxCP2Regs *regs)
{
	u32 flags = 0;

	GTE_LOG("GTE NCLIP\n");

	gteMAC0 = (s32)mac0flags(&flags, (s64)(gteSX0 * (gteSY1 - gteSY2)) +
				gteSX1 * (gteSY2 - gteSY0) +
				gteSX2 * (gteSY0 - gteSY1));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteAVSZ3_(psxCP2Regs *regs)
{
	u32 flags = 0;
	s64 r;

	GTE_LOG("GTE AVSZ3\n");

	r = (s64)gteZSF3 * (gteSZ1 + gteSZ2 + gteSZ3);
	gteMAC0 = (s32)mac0flags(&flags, r);
	gteOTZ = (u16)(limD(&flags, r >> 12));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteAVSZ4_(psxCP2Regs *regs)
{
	u32 flags = 0;
	s64 r;

	GTE_LOG("GTE AVSZ4\n");

	r = (s64)gteZSF4 * (gteSZ0 + gteSZ1 + gteSZ2 + gteSZ3);
	gteMAC0 = (s32)mac0flags(&flags, r);
	gteOTZ = (u16)(limD(&flags, r >> 12));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteSQR(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE SQR\n");

	gteMAC1 = (s32)((gteIR1 * gteIR1) >> shift);
	gteMAC2 = (s32)((gteIR2 * gteIR2) >> shift);
	gteMAC3 = (s32)((gteIR3 * gteIR3) >> shift);
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteFLAG = getFinalFlag(flags);
}

static inline force_inline void runColor1(psxCP2Regs *regs, u32 *flags, int shift, int lm,
	int is_ncc, int is_ncd, int v, int o, int writeback)
{
	s32 mac1, mac2, mac3;
	s32 ir1, ir2, ir3;
	s32 vx, vy, vz;

	vx = VX(v);
	vy = VY(v);
	vz = VZ(v);
	mac1 = ((s64)(gteL11 * vx) + (gteL12 * vy) + (gteL13 * vz)) >> shift;
	mac2 = ((s64)(gteL21 * vx) + (gteL22 * vy) + (gteL23 * vz)) >> shift;
	mac3 = ((s64)(gteL31 * vx) + (gteL32 * vy) + (gteL33 * vz)) >> shift;
	ir1 = limB1(flags, mac1, lm);
	ir2 = limB2(flags, mac2, lm);
	ir3 = limB3(flags, mac3, lm);
	*flags |= gteMAC123f(regs, ir1, ir2, ir3, &gteLR1, &gteRBK, shift);
	mac1 = gteMAC1;
	mac2 = gteMAC2;
	mac3 = gteMAC3;
	if (is_ncc || is_ncd) {
		ir1 = limB1(flags, mac1, lm);
		ir2 = limB2(flags, mac2, lm);
		ir3 = limB3(flags, mac3, lm);
		mac1 = ((s32)gteR * ir1) * 16;
		mac2 = ((s32)gteG * ir2) * 16;
		mac3 = ((s32)gteB * ir3) * 16;
		if (is_ncd) {
			s32 ir0 = gteIR0;
			ir1 = limB1(flags, mac123sub_s12(1, flags, gteRFC, mac1, shift), 0);
			ir2 = limB2(flags, mac123sub_s12(2, flags, gteGFC, mac2, shift), 0);
			ir3 = limB3(flags, mac123sub_s12(3, flags, gteBFC, mac3, shift), 0);
			mac1 = (ir0 * ir1 + (s64)mac1) >> shift;
			mac2 = (ir0 * ir2 + (s64)mac2) >> shift;
			mac3 = (ir0 * ir3 + (s64)mac3) >> shift;
		}
		else {
			mac1 >>= shift;
			mac2 >>= shift;
			mac3 >>= shift;
		}
	}
	fR(o) = (u8)(limC1(flags, mac1 >> 4));
	fG(o) = (u8)(limC2(flags, mac2 >> 4));
	fB(o) = (u8)(limC3(flags, mac3 >> 4));
	fCODE(o) = gteCODE;
	if (writeback) {
		gteMAC1 = (s32)(mac1);
		gteMAC2 = (s32)(mac2);
		gteMAC3 = (s32)(mac3);
		gteIR1 = (s16)(limB1(flags, mac1, lm));
		gteIR2 = (s16)(limB2(flags, mac2, lm));
		gteIR3 = (s16)(limB3(flags, mac3, lm));
		gteFLAG = getFinalFlag(*flags);
	}
	else {
		limBF1(flags, mac1, lm);
		limBF2(flags, mac2, lm);
		limBF3(flags, mac3, lm);
	}
}

static inline void shiftRGB(psxCP2Regs *regs)
{
	gteRGB0 = gteRGB1;
	gteRGB1 = gteRGB2;
	gteCODE2 = gteCODE;
}

static inline void gteNCCS(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCCS\n");

	shiftRGB(regs);
	runColor1(regs, &flags, shift, lm, 1, 0, 0, 2, 1);
}

static inline void gteNCCT(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCCT\n");

	runColor1(regs, &flags, shift, lm, 1, 0, 0, 0, 0);
	runColor1(regs, &flags, shift, lm, 1, 0, 1, 1, 0);
	runColor1(regs, &flags, shift, lm, 1, 0, 2, 2, 1);
}

static inline void gteNCDS(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCDS\n");

	shiftRGB(regs);
	runColor1(regs, &flags, shift, lm, 0, 1, 0, 2, 1);
}

static inline void gteNCDT(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCDT\n");

	runColor1(regs, &flags, shift, lm, 0, 1, 0, 0, 0);
	runColor1(regs, &flags, shift, lm, 0, 1, 1, 1, 0);
	runColor1(regs, &flags, shift, lm, 0, 1, 2, 2, 1);
}

static inline void gteNCS(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCS\n");

	shiftRGB(regs);
	runColor1(regs, &flags, shift, lm, 0, 0, 0, 2, 1);
}

static inline void gteNCT(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE NCT\n");

	runColor1(regs, &flags, shift, lm, 0, 0, 0, 0, 0);
	runColor1(regs, &flags, shift, lm, 0, 0, 1, 1, 0);
	runColor1(regs, &flags, shift, lm, 0, 0, 2, 2, 1);
}

static inline void gteOP(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE OP\n");

	gteMAC1 = (s32)(((gteR22 * gteIR3) - (gteR33 * gteIR2)) >> shift);
	gteMAC2 = (s32)(((gteR33 * gteIR1) - (gteR11 * gteIR3)) >> shift);
	gteMAC3 = (s32)(((gteR11 * gteIR2) - (gteR22 * gteIR1)) >> shift);
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteGPF(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE GPF\n");

	shiftRGB(regs);
	gteMAC1 = (s32)((gteIR0 * gteIR1) >> shift);
	gteMAC2 = (s32)((gteIR0 * gteIR2) >> shift);
	gteMAC3 = (s32)((gteIR0 * gteIR3) >> shift);
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteR2 = (u8)(limC1(&flags, gteMAC1 >> 4));
	gteG2 = (u8)(limC2(&flags, gteMAC2 >> 4));
	gteB2 = (u8)(limC3(&flags, gteMAC3 >> 4));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteGPL(psxCP2Regs *regs, int shift, int lm)
{
	s16 ir0 = gteIR0;
	u32 flags = 0;

	GTE_LOG("GTE GPL\n");

	shiftRGB(regs);
	if (shift) {
		gteMAC1 = (s32)(mac123add_s12(1, &flags, gteMAC1, ir0 * gteIR1, 12));
		gteMAC2 = (s32)(mac123add_s12(2, &flags, gteMAC2, ir0 * gteIR2, 12));
		gteMAC3 = (s32)(mac123add_s12(3, &flags, gteMAC3, ir0 * gteIR3, 12));
	}
	else {
		gteMAC1 += ir0 * gteIR1;
		gteMAC2 += ir0 * gteIR2;
		gteMAC3 += ir0 * gteIR3;
	}
	gteIR1 = (s16)(limB1(&flags, gteMAC1, lm));
	gteIR2 = (s16)(limB2(&flags, gteMAC2, lm));
	gteIR3 = (s16)(limB3(&flags, gteMAC3, lm));
	gteR2 = (u8)(limC1(&flags, gteMAC1 >> 4));
	gteG2 = (u8)(limC2(&flags, gteMAC2 >> 4));
	gteB2 = (u8)(limC3(&flags, gteMAC3 >> 4));
	gteFLAG = getFinalFlag(flags);
}

static inline void runColor2(psxCP2Regs *regs, u32 *flags, int shift, int lm,
	s32 mac1, s32 mac2, s32 mac3, int o, int writeback)
{
	s32 ir1, ir2, ir3;
	s32 ir0 = gteIR0;

	ir1 = limB1(flags, mac123sub_s12(1, flags, gteRFC, mac1, shift), 0);
	ir2 = limB2(flags, mac123sub_s12(2, flags, gteGFC, mac2, shift), 0);
	ir3 = limB3(flags, mac123sub_s12(3, flags, gteBFC, mac3, shift), 0);
	mac1 = gteMAC1 = (s32)((ir0 * ir1 + (s64)mac1) >> shift);
	mac2 = gteMAC2 = (s32)((ir0 * ir2 + (s64)mac2) >> shift);
	mac3 = gteMAC3 = (s32)((ir0 * ir3 + (s64)mac3) >> shift);
	ir1 = limB1(flags, mac1, lm);
	ir2 = limB2(flags, mac2, lm);
	ir3 = limB3(flags, mac3, lm);
	fR(o) = (u8)(limC1(flags, mac1 >> 4));
	fG(o) = (u8)(limC2(flags, mac2 >> 4));
	fB(o) = (u8)(limC3(flags, mac3 >> 4));
	if (writeback) {
		gteMAC1 = (s32)(mac1);
		gteMAC2 = (s32)(mac2);
		gteMAC3 = (s32)(mac3);
		gteIR1 = (s16)(ir1);
		gteIR2 = (s16)(ir2);
		gteIR3 = (s16)(ir3);
		gteFLAG = getFinalFlag(*flags);
	}
}

static inline void gteDCPL(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE DCPL\n");

	s32 mac1 = ((s32)gteR * gteIR1) * 16;
	s32 mac2 = ((s32)gteG * gteIR2) * 16;
	s32 mac3 = ((s32)gteB * gteIR3) * 16;
	shiftRGB(regs);
	runColor2(regs, &flags, shift, lm, mac1, mac2, mac3, 2, 1);
}

static inline void gteDPCS(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE DPCS\n");

	shiftRGB(regs);
	runColor2(regs, &flags, shift, lm, gteR << 16, gteG << 16, gteB << 16, 2, 1);
}

static inline void gteDPCT(psxCP2Regs *regs, int shift, int lm)
{
	s32 mac1, mac2, mac3;
	u8 code0 = gteCODE;
	u32 flags = 0;
	int v;

	GTE_LOG("GTE DPCT\n");

	for (v = 0; v < 3; v++) {
		mac1 = fR(v) << 16;
		mac2 = fG(v) << 16;
		mac3 = fB(v) << 16;
		runColor2(regs, &flags, shift, lm, mac1, mac2, mac3, v, v == 2);
		fCODE(v) = code0;
	}
}

static inline void gteINTPL(psxCP2Regs *regs, int shift, int lm)
{
	u32 flags = 0;

	GTE_LOG("GTE INTPL\n");

	shiftRGB(regs);
	runColor2(regs, &flags, shift, lm, gteIR1 * 4096, gteIR2 * 4096, gteIR3 * 4096, 2, 1);
}

static inline void runColor3(psxCP2Regs *regs, int shift, int lm, int is_cdp)
{
	s32 mac1, mac2, mac3;
	s32 ir1, ir2, ir3;
	u32 flags;

	flags = gteMAC123f(regs, gteIR1, gteIR2, gteIR3, &gteLR1, &gteRBK, shift);
	ir1 = limB1(&flags, gteMAC1, lm);
	ir2 = limB2(&flags, gteMAC2, lm);
	ir3 = limB3(&flags, gteMAC3, lm);
	mac1 = ((s32)gteR * ir1) * 16;
	mac2 = ((s32)gteG * ir2) * 16;
	mac3 = ((s32)gteB * ir3) * 16;
	if (is_cdp) {
		s32 ir0 = gteIR0;
		ir1 = limB1(&flags, mac123sub_s12(1, &flags, gteRFC, mac1, shift), 0);
		ir2 = limB2(&flags, mac123sub_s12(2, &flags, gteGFC, mac2, shift), 0);
		ir3 = limB3(&flags, mac123sub_s12(3, &flags, gteBFC, mac3, shift), 0);
		mac1 = (ir0 * ir1 + (s64)mac1) >> shift;
		mac2 = (ir0 * ir2 + (s64)mac2) >> shift;
		mac3 = (ir0 * ir3 + (s64)mac3) >> shift;
	}
	else {
		mac1 >>= shift;
		mac2 >>= shift;
		mac3 >>= shift;
	}
	gteMAC1 = (s32)(mac1);
	gteMAC2 = (s32)(mac2);
	gteMAC3 = (s32)(mac3);
	gteIR1 = (s16)(limB1(&flags, mac1, lm));
	gteIR2 = (s16)(limB2(&flags, mac2, lm));
	gteIR3 = (s16)(limB3(&flags, mac3, lm));
	gteRGB0 = gteRGB1;
	gteRGB1 = gteRGB2;
	gteCODE2 = gteCODE;
	gteR2 = (u8)(limC1(&flags, mac1 >> 4));
	gteG2 = (u8)(limC2(&flags, mac2 >> 4));
	gteB2 = (u8)(limC3(&flags, mac3 >> 4));
	gteFLAG = getFinalFlag(flags);
}

static inline void gteCC(psxCP2Regs *regs, int shift, int lm)
{
	GTE_LOG("GTE CC\n");
	runColor3(regs, shift, lm, 0);
}

static inline void gteCDP(psxCP2Regs *regs, int shift, int lm)
{
	GTE_LOG("GTE CDP\n");
	runColor3(regs, shift, lm, 1);
}


/* PORT: native register contexts replace emulator-global CPU state and dispatch. */
_Static_assert(sizeof(psxCP2Regs)==sizeof(psxgpu_gte), "GTE wire ABI");
u32 psxgpu_gte_read_data(psxgpu_gte *state,u32 reg) {
    psxCP2Regs local; u32 result;
    if (reg>=32) return 0;
    memcpy(&local,state,sizeof(local)); result=MFC2(&local,(int)reg);
    memcpy(state,&local,sizeof(local)); return result;
}
void psxgpu_gte_write_data(psxgpu_gte *state,u32 reg,u32 value) {
    psxCP2Regs local; if (reg>=32) return;
    memcpy(&local,state,sizeof(local)); MTC2(&local,value,(int)reg); memcpy(state,&local,sizeof(local));
}
u32 psxgpu_gte_read_control(const psxgpu_gte *state,u32 reg) {return reg<32 ? state->control[reg] : 0;}
void psxgpu_gte_write_control(psxgpu_gte *state,u32 reg,u32 value) {
    psxCP2Regs local; if (reg>=32) return;
    memcpy(&local,state,sizeof(local)); CTC2(&local,value,(int)reg); memcpy(state,&local,sizeof(local));
}
int psxgpu_gte_execute(psxgpu_gte *state,u32 code) {
    psxCP2Regs local;
    int shift=(int)GTE_SF(code)*12, lm=(int)GTE_LM(code);
    memcpy(&local,state,sizeof(local));
    switch(code&63u) {
    case 1: gteRTPS(&local,shift,lm); break;
    case 6: gteNCLIP_(&local); break;
    case 12: gteOP(&local,shift,lm); break;
    case 16: gteDPCS(&local,shift,lm); break;
    case 17: gteINTPL(&local,shift,lm); break;
    case 18: gteMVMVA_generic(&local,code); break;
    case 19: gteNCDS(&local,shift,lm); break;
    case 20: gteCDP(&local,shift,lm); break;
    case 22: gteNCDT(&local,shift,lm); break;
    case 27: gteNCCS(&local,shift,lm); break;
    case 28: gteCC(&local,shift,lm); break;
    case 30: gteNCS(&local,shift,lm); break;
    case 32: gteNCT(&local,shift,lm); break;
    case 40: gteSQR(&local,shift,lm); break;
    case 41: gteDCPL(&local,shift,lm); break;
    case 42: gteDPCT(&local,shift,lm); break;
    case 45: gteAVSZ3_(&local); break;
    case 46: gteAVSZ4_(&local); break;
    case 48: gteRTPT(&local,shift,lm); break;
    case 61: gteGPF(&local,shift,lm); break;
    case 62: gteGPL(&local,shift,lm); break;
    case 63: gteNCCT(&local,shift,lm); break;
    default: return -1;
    }
    memcpy(state,&local,sizeof(local)); return 0;
}
