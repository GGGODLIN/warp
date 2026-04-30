#import <AppKit/AppKit.h>

// WarpStatusItem owns an NSStatusItem in the system menu bar and routes its
// menu item clicks back into Rust. The Rust side hands us two opaque context
// pointers — one per menu item — that we pass back through the
// warp_status_item_action_invoked callback.
@interface WarpStatusItem : NSObject {
    NSStatusItem *_statusItem;
    void *_showWarpContext;
    void *_quitWarpContext;
}

- (instancetype)initWithImageData:(NSData *)imageData
                  showWarpContext:(void *)showWarpContext
                  quitWarpContext:(void *)quitWarpContext;

// Removes the status item from the menu bar. After calling this the receiver
// should be deallocated.
- (void)removeFromStatusBar;

// Selectors invoked by NSMenuItem target-action when each menu item fires.
- (void)showWarpClicked:(id)sender;
- (void)quitWarpClicked:(id)sender;

@end

// Creates a new status item and returns it as a retained NSStatusItem-owning
// WarpStatusItem. Caller must balance with warp_status_item_destroy.
WarpStatusItem *warp_status_item_create(NSData *imageData, void *showWarpContext,
                                        void *quitWarpContext);

// Removes from the status bar and releases the WarpStatusItem.
void warp_status_item_destroy(WarpStatusItem *item);
