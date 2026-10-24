// Refuse macOS auto-mount of the update drive while this runs, so macOS never
// writes .fseventsd or ._ files to it. The raw disk stays usable by iapwrite.py.
// build: clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o nomount nomount.c
//            -framework DiskArbitration -framework CoreFoundation
// usage: nomount [VOLUME_NAME ...]   (default POKEMON-2 and HAM MODULE; Ctrl-C to stop)
#include <CoreFoundation/CoreFoundation.h>
#include <DiskArbitration/DiskArbitration.h>
#include <stdio.h>

// Board 2.64 names the drive POKEMON-2; the manual calls it HAM MODULE.
static const char *default_names[] = {"POKEMON-2", "HAM MODULE"};
static CFStringRef targets[16];
static int target_count;

static DADissenterRef approve_mount(DADiskRef disk, void *context) {
    (void)context;
    CFDictionaryRef desc = DADiskCopyDescription(disk);
    if (!desc) return NULL;
    CFStringRef name = CFDictionaryGetValue(desc, kDADiskDescriptionVolumeNameKey);
    int match = 0;
    for (int i = 0; name && i < target_count && !match; i++) {
        match = CFStringCompare(name, targets[i], kCFCompareCaseInsensitive) == kCFCompareEqualTo;
    }
    CFRelease(desc);
    if (!match) return NULL;
    printf("blocked mount of /dev/%s\n", DADiskGetBSDName(disk));
    fflush(stdout);
    return DADissenterCreate(kCFAllocatorDefault, kDAReturnNotPermitted, CFSTR("blocked by nomount"));
}

int main(int argc, char **argv) {
    const char **names = argc > 1 ? (const char **)argv + 1 : default_names;
    int count = argc > 1 ? argc - 1 : 2;
    if (count > 16) {
        fprintf(stderr, "nomount: at most 16 volume names\n");
        return 1;
    }
    for (int i = 0; i < count; i++) {
        targets[target_count++] = CFStringCreateWithCString(kCFAllocatorDefault, names[i], kCFStringEncodingUTF8);
        printf("blocking mounts of %s\n", names[i]);
    }
    DASessionRef session = DASessionCreate(kCFAllocatorDefault);
    DARegisterDiskMountApprovalCallback(session, NULL, approve_mount, NULL);
    DASessionScheduleWithRunLoop(session, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);
    printf("Ctrl-C to stop\n");
    fflush(stdout);
    CFRunLoopRun();
    return 0;
}
