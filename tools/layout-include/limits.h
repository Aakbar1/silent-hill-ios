/* PORT: Compile-only target limits; production uses the platform CRT. */
#ifndef SH_LAYOUT_LIMITS_H
#define SH_LAYOUT_LIMITS_H
#define INT_MAX __INT_MAX__
#define INT_MIN (-INT_MAX-1)
#define SHRT_MAX __SHRT_MAX__
#define SHRT_MIN (-SHRT_MAX-1)
#define UCHAR_MAX 255
#define CHAR_BIT 8
#endif
