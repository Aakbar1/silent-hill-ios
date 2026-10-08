/* SPDX-License-Identifier: GPL-3.0-only */
// PORT: The native iOS sink owns an AVAudioSession playback category. Audio
// remains audible with the Ring/Silent switch set to silent, as game playback
// requires. Preferred device timing never changes the 44.1kHz game clock.
#if defined(__APPLE__)
#include <stdbool.h>
typedef void* Obj;
typedef void* Sel;
extern Obj objc_getClass(const char* name);
extern Sel sel_registerName(const char* name);
extern void objc_msgSend(void);
extern void* objc_autoreleasePoolPush(void);
extern void objc_autoreleasePoolPop(void* pool);
extern Obj AVAudioSessionCategoryPlayback;
int audio_ios_session_start(void) {
    void* pool=objc_autoreleasePoolPush();
    Obj (*get)(Obj,Sel)=(Obj (*)(Obj,Sel))objc_msgSend;
    bool (*category)(Obj,Sel,Obj,Obj*)=(bool (*)(Obj,Sel,Obj,Obj*))objc_msgSend;
    bool (*number)(Obj,Sel,double,Obj*)=(bool (*)(Obj,Sel,double,Obj*))objc_msgSend;
    bool (*active)(Obj,Sel,bool,Obj*)=(bool (*)(Obj,Sel,bool,Obj*))objc_msgSend;
    Obj session=get(objc_getClass("AVAudioSession"),sel_registerName("sharedInstance"));
    Obj error=0;
    int result=1;
    if(session && category(session,sel_registerName("setCategory:error:"),AVAudioSessionCategoryPlayback,&error) &&
       number(session,sel_registerName("setPreferredSampleRate:error:"),44100.0,&error) &&
       number(session,sel_registerName("setPreferredIOBufferDuration:error:"),256.0/44100.0,&error) &&
       active(session,sel_registerName("setActive:error:"),true,&error))result=0;
    objc_autoreleasePoolPop(pool);
    return result;
}
#else
int audio_ios_session_start(void) {return 0;}
#endif
