/* PORT: Compile-only builtin-target declarations; runtime uses its native CRT. */
#ifndef SH_LAYOUT_STRING_H
#define SH_LAYOUT_STRING_H
#include <stddef.h>
void* memcpy(void* destination,const void* source,size_t length);
void* memset(void* destination,int value,size_t length);
#endif
