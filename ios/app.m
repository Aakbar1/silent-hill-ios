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

void sh_ios_status(const char *, double, bool);
static dispatch_queue_t importQueue;

@interface SHController : UIViewController <UIDocumentPickerDelegate>
@property(nonatomic, strong) UILabel *titleLabel;
@property(nonatomic, strong) UILabel *message;
@property(nonatomic, strong) UIButton *choose;
@property(nonatomic, strong) UIActivityIndicatorView *spinner;
@property(nonatomic, strong) UIProgressView *progress;
@property(nonatomic, strong) UIStackView *importer;
@property(nonatomic, strong) UIImageView *game;
@property(nonatomic, strong) NSTimer *timer;
@property(nonatomic) BOOL importing;
@property(nonatomic) BOOL playing;
- (void)importURL:(NSURL *)url;
@end
static SHController *controller;
static NSURL *documentsURL, *supportURL;

@implementation SHController
- (void)viewDidLoad {
    [super viewDidLoad];
    self.view.backgroundColor = [UIColor colorWithRed:0.055 green:0.075 blue:0.09 alpha:1];
    self.game = [[UIImageView alloc] init];
    self.game.contentMode = UIViewContentModeScaleAspectFit;
    self.game.translatesAutoresizingMaskIntoConstraints = NO;
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
        [self.importer.bottomAnchor constraintLessThanOrEqualToAnchor:safe.bottomAnchor constant:-16],
        [self.game.leadingAnchor constraintEqualToAnchor:safe.leadingAnchor],
        [self.game.trailingAnchor constraintEqualToAnchor:safe.trailingAnchor],
        [self.game.topAnchor constraintEqualToAnchor:safe.topAnchor],
        [self.game.bottomAnchor constraintEqualToAnchor:safe.bottomAnchor]
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
        if (image) { controller.game.image = [UIImage imageWithCGImage:image]; CGImageRelease(image); }
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
- (void)sceneWillResignActive:(UIScene *)scene { (void)scene; sh_ios_active(false); }
- (void)sceneDidBecomeActive:(UIScene *)scene { (void)scene; sh_ios_active(true); }
@end

void sh_ios_application_main(void) {
    @autoreleasepool { UIApplicationMain(0, NULL, nil, NSStringFromClass(SHDelegate.class)); }
}
