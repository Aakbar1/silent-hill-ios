// SPDX-License-Identifier: GPL-3.0-only
// Original UIKit importer and presenter. No game art or data is bundled.
#import <UIKit/UIKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#import <AVFoundation/AVFoundation.h>
#import <os/lock.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

extern void sh_ios_initialize(const char *, const char *);
extern void sh_ios_import(const char *);
extern bool sh_ios_wants_import(void);
extern void sh_ios_poll(void);
extern void sh_ios_active(bool);
extern void sh_ios_interrupted(bool);
extern void sh_ios_audio_route_changed(void);
extern void sh_ios_view(float, float, float, float, float, float, float);
extern void sh_ios_touch(uint64_t, uint32_t, float, float);
extern void sh_ios_ready(void);

void sh_ios_status(const char *, double, bool);
void sh_ios_synthetic_touch(uint64_t identity, uint32_t phase, float x, float y) {
    // Data-free fixture only. SHGameView delivers to this exact same Rust FFI.
    sh_ios_touch(identity, phase, x, y);
}
void sh_ios_synthetic_audio_notifications(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        printf("IOS_SMOKE synthetic interruption notification began\n"); fflush(stdout);
        [NSNotificationCenter.defaultCenter postNotificationName:AVAudioSessionInterruptionNotification
            object:AVAudioSession.sharedInstance userInfo:@{AVAudioSessionInterruptionTypeKey:@(AVAudioSessionInterruptionTypeBegan)}];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 300 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
            [NSNotificationCenter.defaultCenter postNotificationName:AVAudioSessionInterruptionNotification
                object:AVAudioSession.sharedInstance userInfo:@{AVAudioSessionInterruptionTypeKey:@(AVAudioSessionInterruptionTypeEnded),
                    AVAudioSessionInterruptionOptionKey:@(AVAudioSessionInterruptionOptionShouldResume)}];
        });
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 1500 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
            printf("IOS_SMOKE synthetic route notification\n"); fflush(stdout);
            [NSNotificationCenter.defaultCenter postNotificationName:AVAudioSessionRouteChangeNotification
                object:AVAudioSession.sharedInstance userInfo:@{AVAudioSessionRouteChangeReasonKey:@(AVAudioSessionRouteChangeReasonNewDeviceAvailable)}];
        });
    });
}
static dispatch_queue_t importQueue;
static BOOL sceneActive = YES, audioInterrupted;
static UIBackgroundTaskIdentifier pauseTask = UIBackgroundTaskInvalid;

// Touch identity lasts from began through ended/cancelled. Coordinates and safe
// areas are in the SAME logical view used by the display-copy overlay.
@interface SHGameView : UIImageView
@property(nonatomic, strong) NSMutableDictionary<NSValue *, NSNumber *> *contacts;
@property(nonatomic) uint64_t nextContact;
@end
@implementation SHGameView
- (instancetype)init {
    self = [super init];
    if (self) {
        self.userInteractionEnabled = YES;
        self.multipleTouchEnabled = YES;
        self.contacts = [NSMutableDictionary dictionary];
        self.contentMode = UIViewContentModeScaleToFill;
    }
    return self;
}
- (void)deliver:(NSSet<UITouch *> *)touches phase:(uint32_t)phase {
    NSArray<UITouch *> *ordered = [touches.allObjects sortedArrayUsingComparator:^NSComparisonResult(UITouch *a, UITouch *b) {
        return a.timestamp < b.timestamp ? NSOrderedAscending : a.timestamp > b.timestamp ? NSOrderedDescending : NSOrderedSame;
    }];
    for (UITouch *touch in ordered) {
        NSValue *key = [NSValue valueWithNonretainedObject:touch];
        if (phase == 0) { self.contacts[key] = @(++self.nextContact); }
        NSNumber *identity = self.contacts[key];
        if (!identity) { continue; }
        CGPoint point = [touch locationInView:self];
        sh_ios_touch(identity.unsignedLongLongValue, phase, (float)point.x, (float)point.y);
        if (phase >= 2) { [self.contacts removeObjectForKey:key]; }
    }
}
- (void)touchesBegan:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event { (void)event; [self deliver:touches phase:0]; }
- (void)touchesMoved:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event { (void)event; [self deliver:touches phase:1]; }
- (void)touchesEnded:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event { (void)event; [self deliver:touches phase:2]; }
- (void)touchesCancelled:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event { (void)event; [self deliver:touches phase:3]; }
@end

@interface SHController : UIViewController <UIDocumentPickerDelegate>
@property(nonatomic, strong) UILabel *titleLabel;
@property(nonatomic, strong) UILabel *message;
@property(nonatomic, strong) UIButton *choose;
@property(nonatomic, strong) UIActivityIndicatorView *spinner;
@property(nonatomic, strong) UIProgressView *progress;
@property(nonatomic, strong) UIStackView *importer;
@property(nonatomic, strong) SHGameView *game;
@property(nonatomic, strong) NSTimer *timer;
@property(nonatomic) BOOL importing;
@property(nonatomic) BOOL playing;
- (void)importURL:(NSURL *)url;
@end
static SHController *controller;
static NSURL *documentsURL, *supportURL;

void sh_ios_refresh_view(void) {
    [controller.view setNeedsLayout];
    [controller.view layoutIfNeeded];
    // Publish even if Auto Layout already ran before Rust initialized STATE.
    [controller viewDidLayoutSubviews];
}

@implementation SHController
- (void)viewDidLoad {
    [super viewDidLoad];
    self.view.backgroundColor = [UIColor colorWithRed:0.055 green:0.075 blue:0.09 alpha:1];
    self.game = [[SHGameView alloc] init];
    self.game.hidden = YES;
    [self.view addSubview:self.game];
    self.titleLabel = [[UILabel alloc] init];
    self.titleLabel.text = @"Import your Silent Hill (USA) disc image (.bin)";
    self.titleLabel.font = [UIFont systemFontOfSize:24 weight:UIFontWeightSemibold];
    self.titleLabel.textColor = UIColor.whiteColor;
    self.titleLabel.numberOfLines = 0;
    self.titleLabel.textAlignment = NSTextAlignmentCenter;
    self.titleLabel.accessibilityIdentifier = @"disc-import-title";
    self.message = [[UILabel alloc] init];
    self.message.text = @"Use your own US v1.1 disc. Choose a file, or copy it into this app's folder in Files. Your original stays untouched.";
    self.message.font = [UIFont systemFontOfSize:17];
    self.message.textColor = [UIColor colorWithWhite:0.85 alpha:1];
    self.message.numberOfLines = 0;
    self.message.textAlignment = NSTextAlignmentCenter;
    self.choose = [UIButton buttonWithType:UIButtonTypeSystem];
    [self.choose setTitle:@"Choose .bin in Files" forState:UIControlStateNormal];
    self.choose.titleLabel.font = [UIFont systemFontOfSize:20 weight:UIFontWeightSemibold];
    self.choose.accessibilityIdentifier = @"disc-import-choose";
    [self.choose addTarget:self action:@selector(chooseDisc) forControlEvents:UIControlEventTouchUpInside];
    self.spinner = [[UIActivityIndicatorView alloc] initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleMedium];
    self.spinner.color = UIColor.whiteColor;
    self.spinner.hidesWhenStopped = YES;
    self.progress = [[UIProgressView alloc] initWithProgressViewStyle:UIProgressViewStyleDefault];
    self.progress.hidden = YES;
    self.importer = [[UIStackView alloc] initWithArrangedSubviews:@[self.titleLabel, self.message, self.choose, self.spinner, self.progress]];
    self.importer.axis = UILayoutConstraintAxisVertical;
    self.importer.spacing = 18;
    self.importer.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:self.importer];
    UILayoutGuide *safe = self.view.safeAreaLayoutGuide;
    [NSLayoutConstraint activateConstraints:@[
        [self.importer.centerXAnchor constraintEqualToAnchor:safe.centerXAnchor],
        [self.importer.centerYAnchor constraintEqualToAnchor:safe.centerYAnchor],
        [self.importer.widthAnchor constraintLessThanOrEqualToConstant:620],
        [self.importer.leadingAnchor constraintGreaterThanOrEqualToAnchor:safe.leadingAnchor constant:24],
        [self.importer.trailingAnchor constraintLessThanOrEqualToAnchor:safe.trailingAnchor constant:-24],
        [self.importer.topAnchor constraintGreaterThanOrEqualToAnchor:safe.topAnchor constant:16],
        [self.importer.bottomAnchor constraintLessThanOrEqualToAnchor:safe.bottomAnchor constant:-16]
    ]];
    self.timer = [NSTimer scheduledTimerWithTimeInterval:1 repeats:YES block:^(NSTimer *timer) {
        (void)timer;
        sh_ios_poll();
    }];
}
- (void)viewDidAppear:(BOOL)animated {
    [super viewDidAppear:animated];
    // CI additionally checks the captured pixels with Vision OCR.
    if (!self.playing) { printf("importer screen appeared\n"); fflush(stdout); }
}
- (void)viewDidLayoutSubviews {
    [super viewDidLayoutSubviews];
    // PORT: Original display aspect is 320:224 even for 448-line scanouts.
    // Keep touch/overlay geometry inside the actual safe game rectangle.
    CGRect safe = self.view.safeAreaLayoutGuide.layoutFrame;
    CGFloat width = MIN(safe.size.width, safe.size.height * 320.0 / 224.0);
    CGFloat height = width * 224.0 / 320.0;
    self.game.frame = CGRectMake(CGRectGetMidX(safe)-width/2, CGRectGetMidY(safe)-height/2, width, height);
    UIEdgeInsets insets = self.game.safeAreaInsets;
    sh_ios_view((float)width, (float)height, (float)insets.top, (float)insets.right,
                (float)insets.bottom, (float)insets.left, (float)self.view.window.screen.scale);
    printf("resize %.0fx%.0f\n", self.view.bounds.size.width, self.view.bounds.size.height);
    fflush(stdout);
}
- (UIInterfaceOrientationMask)supportedInterfaceOrientations { return UIInterfaceOrientationMaskLandscape; }
- (BOOL)prefersStatusBarHidden { return YES; }
- (BOOL)prefersHomeIndicatorAutoHidden { return YES; }
- (UIRectEdge)preferredScreenEdgesDeferringSystemGestures { return UIRectEdgeAll; }
- (void)chooseDisc {
    if (!sh_ios_wants_import() || self.importing) { return; }
    // public.data keeps BINs with provider-specific UTIs selectable. Rust checks
    // the extension and psxdisc verifies the actual release, never the name.
    UIDocumentPickerViewController *picker = [[UIDocumentPickerViewController alloc] initForOpeningContentTypes:@[UTTypeData] asCopy:NO];
    picker.allowsMultipleSelection = NO;
    picker.delegate = self;
    [self presentViewController:picker animated:YES completion:nil];
}
- (void)documentPicker:(UIDocumentPickerViewController *)picker didPickDocumentsAtURLs:(NSArray<NSURL *> *)urls {
    (void)picker;
    if (urls.count == 1) { [self importURL:urls.firstObject]; }
}
- (void)importURL:(NSURL *)url {
    if (!sh_ios_wants_import() || self.importing) { return; }
    self.importing = YES;
    self.choose.enabled = NO;
    sh_ios_status("Opening your file...", -1, true);
    dispatch_async(importQueue, ^{
        @autoreleasepool {
            BOOL scoped = [url startAccessingSecurityScopedResource];
            @try {
                NSError *error = nil;
                NSFileCoordinator *coordinator = [[NSFileCoordinator alloc] initWithFilePresenter:nil];
                [coordinator coordinateReadingItemAtURL:url options:0 error:&error byAccessor:^(NSURL *localURL) {
                    sh_ios_import(localURL.fileSystemRepresentation);
                }];
                if (error) { sh_ios_status("Could not read this file. Finish its download/copy and choose it again.", -1, false); }
            } @finally {
                if (scoped) { [url stopAccessingSecurityScopedResource]; }
            }
        }
        dispatch_async(dispatch_get_main_queue(), ^{ self.importing = NO; });
    });
}
@end

void sh_ios_import_local(const char *path) {
    NSString *copied = [NSString stringWithUTF8String:path];
    [controller importURL:[NSURL fileURLWithPath:copied]];
}

void sh_ios_status(const char *text, double progress, bool busy) {
    // Rust worker threads do not have UIKit's implicit autorelease pool.
    @autoreleasepool {
    NSString *copied = [NSString stringWithUTF8String:text];
    dispatch_async(dispatch_get_main_queue(), ^{
        if (controller.playing && busy) { return; }
        controller.message.text = copied;
        controller.choose.enabled = !busy;
        controller.progress.hidden = progress < 0;
        controller.progress.progress = (float)progress;
        if (busy) { [controller.spinner startAnimating]; } else { [controller.spinner stopAnimating]; }
        if (controller.playing && !busy) {
            controller.importer.hidden = NO;
            controller.titleLabel.text = @"The native host stopped";
            controller.choose.hidden = YES;
        }
    });
    }
}

void sh_ios_game_started(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        controller.playing = YES;
        controller.importer.hidden = YES;
        [controller.spinner stopAnimating];
        controller.game.hidden = NO;
        [controller.view setNeedsLayout];
        [controller.view layoutIfNeeded];
        sh_ios_ready();
    });
}

// Worker acknowledgement comes only at a safe callback/tick boundary. Saves
// already use synchronous sync+atomic rename; no UI-thread save writes exist.
void sh_ios_worker_paused(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!sceneActive && !audioInterrupted) {
            NSError *error = nil;
            if (![AVAudioSession.sharedInstance setActive:NO withOptions:AVAudioSessionSetActiveOptionNotifyOthersOnDeactivation error:&error]) {
                printf("IOS_AUDIO deactivate failed: %s\n", error.localizedDescription.UTF8String);
            }
        }
        if (pauseTask != UIBackgroundTaskInvalid) {
            [UIApplication.sharedApplication endBackgroundTask:pauseTask];
            pauseTask = UIBackgroundTaskInvalid;
        }
        printf("IOS_LIFECYCLE pause acknowledged\n"); fflush(stdout);
    });
}

// A bounded latest-frame mailbox: UIKit may fall behind without accumulating
// unbounded frame data or work items. Backgrounding pauses the Rust presenter.
static os_unfair_lock frameLock = OS_UNFAIR_LOCK_INIT;
static NSData *latestFrame;
static uint32_t frameWidth, frameHeight;
static BOOL frameScheduled;
void sh_ios_frame(const uint8_t *rgba, uint32_t width, uint32_t height) {
    @autoreleasepool {
    NSData *copy = [NSData dataWithBytes:rgba length:(NSUInteger)width * height * 4];
    os_unfair_lock_lock(&frameLock);
    latestFrame = copy;
    frameWidth = width; frameHeight = height;
    BOOL schedule = !frameScheduled;
    frameScheduled = YES;
    os_unfair_lock_unlock(&frameLock);
    if (!schedule) { return; }
    dispatch_async(dispatch_get_main_queue(), ^{
        os_unfair_lock_lock(&frameLock);
        NSData *data = latestFrame;
        uint32_t w = frameWidth, h = frameHeight;
        latestFrame = nil; frameScheduled = NO;
        os_unfair_lock_unlock(&frameLock);
        CGColorSpaceRef color = CGColorSpaceCreateDeviceRGB();
        CGDataProviderRef provider = CGDataProviderCreateWithCFData((__bridge CFDataRef)data);
        CGImageRef image = CGImageCreate(w, h, 8, 32, (size_t)w * 4, color, kCGBitmapByteOrderDefault | kCGImageAlphaLast, provider, NULL, false, kCGRenderingIntentDefault);
        if (image) {
            controller.game.image = [UIImage imageWithCGImage:image]; CGImageRelease(image);
            static BOOL first = YES;
            if (first) { first = NO; printf("IOS_PRESENT UIImage installed %ux%u\n", w, h); fflush(stdout); }
        }
        CGDataProviderRelease(provider); CGColorSpaceRelease(color);
    });
    }
}

@interface SHDelegate : UIResponder <UIApplicationDelegate>
@end
@interface SHSceneDelegate : UIResponder <UIWindowSceneDelegate>
@property(nonatomic, strong) UIWindow *window;
@end
@implementation SHDelegate
- (void)audioInterruption:(NSNotification *)notification {
    dispatch_async(dispatch_get_main_queue(), ^{
        NSUInteger type = [notification.userInfo[AVAudioSessionInterruptionTypeKey] unsignedIntegerValue];
        if (type == AVAudioSessionInterruptionTypeBegan) {
            audioInterrupted = YES;
            sh_ios_interrupted(true);
        } else {
            NSUInteger options = [notification.userInfo[AVAudioSessionInterruptionOptionKey] unsignedIntegerValue];
            if (options & AVAudioSessionInterruptionOptionShouldResume) {
                audioInterrupted = NO;
                sh_ios_interrupted(false);
            } else { printf("IOS_AUDIO interruption ended; waiting for manual foreground\n"); }
        }
    });
}
- (void)audioRoute:(NSNotification *)notification {
    printf("IOS_AUDIO notification=%s\n", notification.name.UTF8String); fflush(stdout);
    if ([notification.name isEqualToString:AVAudioSessionRouteChangeNotification] &&
        [notification.userInfo[AVAudioSessionRouteChangeReasonKey] unsignedIntegerValue] == AVAudioSessionRouteChangeReasonCategoryChange) { return; }
    sh_ios_audio_route_changed();
}
- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)options {
    (void)application; (void)options;
    NSFileManager *fm = NSFileManager.defaultManager;
    NSURL *documents = [fm URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask].firstObject;
    NSURL *support = [[fm URLsForDirectory:NSApplicationSupportDirectory inDomains:NSUserDomainMask].firstObject URLByAppendingPathComponent:@"SilentHillPort" isDirectory:YES];
    documentsURL = documents; supportURL = support;
    NSError *error = nil;
    [fm createDirectoryAtURL:documents withIntermediateDirectories:YES attributes:nil error:&error];
    if (error) { NSLog(@"Documents directory unavailable: %@", error); return NO; }
    NSURL *log = [documents URLByAppendingPathComponent:@"game.log"];
    NSDictionary *attributes = [fm attributesOfItemAtPath:log.path error:NULL];
    if ([attributes fileSize] > 2 * 1024 * 1024) {
        NSURL *previous = [documents URLByAppendingPathComponent:@"game.previous.log"];
        [fm removeItemAtURL:previous error:NULL];
        [fm moveItemAtURL:log toURL:previous error:NULL];
    }
    if (!freopen(log.fileSystemRepresentation, "a", stdout) || !freopen(log.fileSystemRepresentation, "a", stderr)) { NSLog(@"Could not open Documents/game.log"); return NO; }
    setvbuf(stdout, NULL, _IOLBF, 0); setvbuf(stderr, NULL, _IOLBF, 0);
    printf("UIKit real-host entry; no bundled disc\n");
    // PORT: Playback is the requested media category; Apple specifies it plays
    // even with Ring/Silent set to silent. No background-audio entitlement.
    AVAudioSession *audio = AVAudioSession.sharedInstance;
    if (![audio setCategory:AVAudioSessionCategoryPlayback mode:AVAudioSessionModeDefault options:0 error:&error] || ![audio setActive:YES error:&error]) {
        printf("Audio session setup failed\n");
    }
    NSNotificationCenter *notifications = NSNotificationCenter.defaultCenter;
    [notifications addObserver:self selector:@selector(audioInterruption:) name:AVAudioSessionInterruptionNotification object:nil];
    [notifications addObserver:self selector:@selector(audioRoute:) name:AVAudioSessionRouteChangeNotification object:nil];
    [notifications addObserver:self selector:@selector(audioRoute:) name:AVAudioSessionMediaServicesWereResetNotification object:nil];
    importQueue = dispatch_queue_create("com.bramley.silenthillport.import", DISPATCH_QUEUE_SERIAL);
    return YES;
}
- (UISceneConfiguration *)application:(UIApplication *)application configurationForConnectingSceneSession:(UISceneSession *)session options:(UISceneConnectionOptions *)options {
    (void)application; (void)options;
    UISceneConfiguration *configuration = [[UISceneConfiguration alloc] initWithName:@"SilentHillPort" sessionRole:session.role];
    configuration.delegateClass = SHSceneDelegate.class;
    return configuration;
}
@end

@implementation SHSceneDelegate
- (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session options:(UISceneConnectionOptions *)options {
    (void)session;
    if (![scene isKindOfClass:UIWindowScene.class]) { return; }
    controller = [[SHController alloc] init];
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = controller;
    [self.window makeKeyAndVisible];
    sh_ios_initialize(documentsURL.fileSystemRepresentation, supportURL.fileSystemRepresentation);
    [controller.view setNeedsLayout];
    [controller.view layoutIfNeeded];
    for (UIOpenURLContext *context in options.URLContexts) {
        if (context.URL.isFileURL) { [controller importURL:context.URL]; break; }
    }
}
- (void)scene:(UIScene *)scene openURLContexts:(NSSet<UIOpenURLContext *> *)contexts {
    (void)scene;
    for (UIOpenURLContext *context in contexts) {
        if (context.URL.isFileURL) { [controller importURL:context.URL]; break; }
    }
}
- (void)sceneWillResignActive:(UIScene *)scene {
    (void)scene; sceneActive = NO;
    [controller.game.contacts removeAllObjects];
    if (pauseTask == UIBackgroundTaskInvalid) {
        pauseTask = [UIApplication.sharedApplication beginBackgroundTaskWithName:@"Finish current native save/tick" expirationHandler:^{
            printf("IOS_LIFECYCLE pause deadline expired\n"); fflush(stdout);
            if (pauseTask != UIBackgroundTaskInvalid) {
                [UIApplication.sharedApplication endBackgroundTask:pauseTask];
                pauseTask = UIBackgroundTaskInvalid;
            }
        }];
    }
    sh_ios_active(false);
    if (!controller.playing) { sh_ios_worker_paused(); }
}
- (void)sceneDidBecomeActive:(UIScene *)scene {
    (void)scene; sceneActive = YES;
    // Explicit foreground is the player's resume after a non-resuming call.
    audioInterrupted = NO; sh_ios_interrupted(false);
    sh_ios_active(true);
    [controller.view setNeedsLayout];
}
@end

void sh_ios_application_main(void) {
    // UIApplicationMain requires a non-null argv; Rust owns the real process arguments.
    static char name[] = "SilentHillPort";
    static char *argv[] = {name, NULL};
    @autoreleasepool { UIApplicationMain(1, argv, nil, NSStringFromClass(SHDelegate.class)); }
}
