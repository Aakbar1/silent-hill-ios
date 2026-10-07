/* PORT: Compile-only declarations; production uses the platform CRT. */
#ifndef SH_LAYOUT_STDIO_H
#define SH_LAYOUT_STDIO_H
#include <stddef.h>
typedef struct ShCompileOnlyFile FILE;
extern FILE* stderr;
extern FILE* stdout;
int fflush(FILE*);
int printf(const char*,...) __attribute__((format(printf,1,2)));
int fprintf(FILE*,const char*,...) __attribute__((format(printf,2,3)));
int puts(const char*);
#endif
