/* PORT: Compile-only declarations; production uses the platform CRT. */
#ifndef SH_LAYOUT_STDLIB_H
#define SH_LAYOUT_STDLIB_H
#include <stddef.h>
int abs(int);
void* malloc(size_t);
void free(void*);
#endif
