/* PORT: Syntax checking only. No CRT layout or runtime ABI claim. */
#ifndef SH_LAYOUT_SETJMP_H
#define SH_LAYOUT_SETJMP_H
typedef struct { unsigned long opaque[64]; } jmp_buf[1];
int setjmp(jmp_buf);
void longjmp(jmp_buf,int) __attribute__((noreturn));
#endif
