#include "TargetConditionals.h"
#if TARGET_OS_IPHONE || TARGET_IPHONE_SIMULATOR
    #import <UIKit/UIKit.h>
#else
    #import <AppKit/AppKit.h>
#endif
#import <Foundation/NSFileManager.h>

CFArrayRef macos_get_disks() {
    return (__bridge CFArrayRef)[[NSFileManager defaultManager] mountedVolumeURLsIncludingResourceValuesForKeys:nil options:0];
}
