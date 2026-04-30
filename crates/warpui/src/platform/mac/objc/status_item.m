#import "status_item.h"

// Implemented in Rust (status_item.rs).
void warp_status_item_action_invoked(void *context);
void warp_status_item_context_freed(void *context);

@implementation WarpStatusItem

- (instancetype)initWithImageData:(NSData *)imageData
                  showWarpContext:(void *)showWarpContext
                  quitWarpContext:(void *)quitWarpContext {
    self = [super init];
    if (self) {
        _showWarpContext = showWarpContext;
        _quitWarpContext = quitWarpContext;

        _statusItem = [[[NSStatusBar systemStatusBar]
            statusItemWithLength:NSVariableStatusItemLength] retain];
        NSLog(@"[WarpStatusItem] created NSStatusItem=%p button=%p", _statusItem,
              _statusItem.button);

        if (imageData != nil) {
            NSImage *image = [[[NSImage alloc] initWithData:imageData] autorelease];
            if (image != nil) {
                // Template image so macOS adapts to dark/light menu bar automatically.
                [image setTemplate:YES];
                [_statusItem.button setImage:image];
            }
        }
        if (_statusItem.button.image == nil) {
            // Fallback to a plain title so the user can still find/click the
            // status item even if the image data is missing or invalid.
            [_statusItem.button setTitle:@"Warp"];
            NSLog(@"[WarpStatusItem] set title=Warp; button.title=%@ visible=%d",
                  _statusItem.button.title, _statusItem.button.window != nil);
        }

        NSMenu *menu = [[[NSMenu alloc] initWithTitle:@""] autorelease];

        NSMenuItem *showItem = [[[NSMenuItem alloc] initWithTitle:@"Show Warp"
                                                           action:@selector(showWarpClicked:)
                                                    keyEquivalent:@""] autorelease];
        [showItem setTarget:self];
        [menu addItem:showItem];

        NSMenuItem *quitItem = [[[NSMenuItem alloc] initWithTitle:@"Quit Warp"
                                                           action:@selector(quitWarpClicked:)
                                                    keyEquivalent:@""] autorelease];
        [quitItem setTarget:self];
        [menu addItem:quitItem];

        [_statusItem setMenu:menu];
        NSLog(@"[WarpStatusItem] init done; statusItem.length=%f button.frame=%@",
              _statusItem.length, NSStringFromRect(_statusItem.button.frame));
    }
    return self;
}

- (void)removeFromStatusBar {
    if (_statusItem != nil) {
        [[NSStatusBar systemStatusBar] removeStatusItem:_statusItem];
        [_statusItem release];
        _statusItem = nil;
    }
}

- (void)dealloc {
    if (_statusItem != nil) {
        [[NSStatusBar systemStatusBar] removeStatusItem:_statusItem];
        [_statusItem release];
        _statusItem = nil;
    }
    if (_showWarpContext != NULL) {
        warp_status_item_context_freed(_showWarpContext);
        _showWarpContext = NULL;
    }
    if (_quitWarpContext != NULL) {
        warp_status_item_context_freed(_quitWarpContext);
        _quitWarpContext = NULL;
    }
    [super dealloc];
}

- (void)showWarpClicked:(id)sender {
    (void)sender;
    if (_showWarpContext != NULL) {
        warp_status_item_action_invoked(_showWarpContext);
    }
}

- (void)quitWarpClicked:(id)sender {
    (void)sender;
    if (_quitWarpContext != NULL) {
        warp_status_item_action_invoked(_quitWarpContext);
    }
}

@end

WarpStatusItem *warp_status_item_create(NSData *imageData, void *showWarpContext,
                                        void *quitWarpContext) {
    return [[WarpStatusItem alloc] initWithImageData:imageData
                                     showWarpContext:showWarpContext
                                     quitWarpContext:quitWarpContext];
}

void warp_status_item_destroy(WarpStatusItem *item) {
    if (item != nil) {
        [item removeFromStatusBar];
        [item release];
    }
}
