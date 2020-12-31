#import <AppKit/AppKit.h>
#import <Foundation/NSFileManager.h>

CFArrayRef macos_get_disks() {
    return (__bridge CFArrayRef)[[NSFileManager defaultManager] mountedVolumeURLsIncludingResourceValuesForKeys:nil options:0];
}
