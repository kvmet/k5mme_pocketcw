// Installs one HamCube update file through the device's update drive, from macOS.
// build: clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o pocketcw-flash pocketcw-flash.c
//            -framework DiskArbitration -framework CoreFoundation
// usage: pocketcw-flash FILE   (A000014.bin, F0000C0.bin, ...)
//
// The bootloader routes file data by the name in the root directory entry, so the entry must
// reach the device before the data, as Windows writes it: directory entry, data, FAT, directory
// entry again. The macOS FAT driver writes the data first, so this program blocks the mount of
// the update drive and writes the raw disk itself.
#include <CoreFoundation/CoreFoundation.h>
#include <DiskArbitration/DiskArbitration.h>
#include <ctype.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <spawn.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

#define SECTOR 2048
// MCU flash below 0x14 KB holds the vendor bootloader, the only recovery path without a probe.
#define APP_FIRST_KB 0x14
#define FLASH_KB 256
#define WAIT_S 180
// Success shows for about 1 s, so poll well inside that.
#define POLL_S 0.4

extern char **environ;

// Board 2.64 names the drive POKEMON-2; the vendor manual calls it HAM MODULE.
static const char *drive_names[] = {"POKEMON-2", "HAM MODULE"};
static char found_disk[64];

static void fail(const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    fprintf(stderr, "\nerror: ");
    vfprintf(stderr, fmt, args);
    fprintf(stderr, "\n");
    va_end(args);
    exit(1);
}

static uint16_t le16(const uint8_t *p) { return p[0] | p[1] << 8; }

static void put_le16(uint8_t *p, uint16_t v) {
    p[0] = v;
    p[1] = v >> 8;
}

static void put_le32(uint8_t *p, uint32_t v) {
    put_le16(p, v);
    put_le16(p + 2, v >> 16);
}

static int is_drive_name(const char *name) {
    for (size_t i = 0; i < sizeof drive_names / sizeof *drive_names; i++) {
        if (strcasecmp(name, drive_names[i]) == 0) return 1;
    }
    return 0;
}

// Returns the 8-character base of an update file name, such as "A000014", or exits.
static void update_name(const char *path, char base[8]) {
    char copy[1024];
    snprintf(copy, sizeof copy, "%s", path);
    const char *name = basename(copy);
    int ok = strlen(name) == 11 && (toupper(name[0]) == 'A' || toupper(name[0]) == 'F') &&
             strcasecmp(name + 7, ".BIN") == 0;
    for (int i = 1; ok && i < 7; i++) ok = isxdigit((unsigned char)name[i]);
    if (!ok) fail("%s is not an update file name (A000014.bin, F0000C0.bin, ...)", path);
    for (int i = 0; i < 7; i++) base[i] = toupper(name[i]);
    base[7] = 0;
}

static uint8_t *read_file(const char *path, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) fail("cannot open %s: %s", path, strerror(errno));
    fseek(f, 0, SEEK_END);
    *len = ftell(f);
    rewind(f);
    uint8_t *data = malloc(*len ? *len : 1);
    if (fread(data, 1, *len, f) != *len) fail("cannot read %s", path);
    fclose(f);
    if (*len == 0) fail("%s is empty", path);
    return data;
}

static void check_range(const char *path, const char *base, size_t len) {
    if (base[0] != 'A') return;
    long offset_kb = strtol(base + 1, NULL, 16);
    if (offset_kb < APP_FIRST_KB)
        fail("%s would overwrite the bootloader; MCU files start at A%06X", path, APP_FIRST_KB);
    if (offset_kb * 1024 + len > FLASH_KB * 1024)
        fail("%s (%zu bytes) runs past the end of MCU flash", path, len);
}

static DADissenterRef approve_mount(DADiskRef disk, void *context) {
    (void)context;
    CFDictionaryRef desc = DADiskCopyDescription(disk);
    if (!desc) return NULL;
    CFStringRef cf_name = CFDictionaryGetValue(desc, kDADiskDescriptionVolumeNameKey);
    char name[64] = "";
    if (cf_name) CFStringGetCString(cf_name, name, sizeof name, kCFStringEncodingUTF8);
    CFRelease(desc);
    if (!is_drive_name(name)) return NULL;
    snprintf(found_disk, sizeof found_disk, "%s", DADiskGetBSDName(disk));
    CFRunLoopStop(CFRunLoopGetCurrent());
    return DADissenterCreate(kCFAllocatorDefault, kDAReturnNotPermitted, CFSTR("blocked by pocketcw-flash"));
}

// Runs the run loop, so mount requests get their answer, until the drive appears or the time
// runs out. Returns 1 when the drive appeared since found_disk was last cleared.
static int wait_for_drive(double seconds) {
    CFAbsoluteTime end = CFAbsoluteTimeGetCurrent() + seconds;
    while (!found_disk[0] && CFAbsoluteTimeGetCurrent() < end) {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, end - CFAbsoluteTimeGetCurrent(), true);
    }
    return found_disk[0] != 0;
}

// A mounted drive has already had macOS files written to it, which the update can't share.
static void check_not_mounted(void) {
    DIR *volumes = opendir("/Volumes");
    if (!volumes) return;
    struct dirent *entry;
    while ((entry = readdir(volumes))) {
        if (is_drive_name(entry->d_name))
            fail("the update drive is already mounted. Eject it in Finder, unplug the device, then run this again.");
    }
    closedir(volumes);
}

// Opens the raw disk read/write through authopen, which shows the macOS password dialog and
// passes the open file back over a socket.
static int authopen_rw(const char *path) {
    int sv[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) != 0) fail("socketpair: %s", strerror(errno));
    posix_spawn_file_actions_t actions;
    posix_spawn_file_actions_init(&actions);
    posix_spawn_file_actions_adddup2(&actions, sv[1], STDOUT_FILENO);
    posix_spawn_file_actions_addclose(&actions, sv[0]);
    char flags[8];
    snprintf(flags, sizeof flags, "%d", O_RDWR);
    char *argv[] = {"authopen", "-stdoutpipe", "-o", flags, (char *)path, NULL};
    pid_t pid;
    int err = posix_spawn(&pid, "/usr/libexec/authopen", &actions, NULL, argv, environ);
    posix_spawn_file_actions_destroy(&actions);
    close(sv[1]);
    if (err) fail("cannot start authopen: %s", strerror(err));

    char byte;
    struct iovec iov = {&byte, 1};
    union {
        struct cmsghdr header;
        char space[CMSG_SPACE(sizeof(int))];
    } control;
    struct msghdr msg = {.msg_iov = &iov, .msg_iovlen = 1, .msg_control = &control, .msg_controllen = sizeof control};
    ssize_t n = recvmsg(sv[0], &msg, 0);
    close(sv[0]);
    waitpid(pid, NULL, 0);
    struct cmsghdr *cmsg = CMSG_FIRSTHDR(&msg);
    if (n <= 0 || !cmsg || cmsg->cmsg_level != SOL_SOCKET || cmsg->cmsg_type != SCM_RIGHTS)
        fail("macOS did not allow writing to the device. Run this again, and type your Mac login password when macOS asks.");
    int fd;
    memcpy(&fd, CMSG_DATA(cmsg), sizeof fd);
    return fd;
}

static void read_sector(int fd, off_t offset, uint8_t *buf) {
    if (pread(fd, buf, SECTOR, offset) != SECTOR) fail("cannot read the device: %s", strerror(errno));
}

static void write_bytes(int fd, const uint8_t *buf, size_t len, off_t offset) {
    if (pwrite(fd, buf, len, offset) != (ssize_t)len) fail("cannot write the device: %s", strerror(errno));
}

// Returns the byte offset of the root directory.
static off_t write_update(int fd, const char *base, const uint8_t *payload, size_t len) {
    uint8_t boot[SECTOR];
    read_sector(fd, 0, boot);
    unsigned bytes_per_sector = le16(boot + 11), sectors_per_cluster = boot[13];
    unsigned reserved = le16(boot + 14), fat_count = boot[16], root_entries = le16(boot + 17);
    unsigned fat_sectors = le16(boot + 22);
    if (bytes_per_sector != SECTOR || memcmp(boot + 54, "FAT16   ", 8) != 0)
        fail("unexpected boot sector (%u B sectors, not FAT16)", bytes_per_sector);
    off_t root = (off_t)(reserved + fat_count * fat_sectors) * SECTOR;
    off_t data = root + root_entries * 32;
    size_t cluster_bytes = sectors_per_cluster * SECTOR;

    uint8_t root_sector[SECTOR];
    read_sector(fd, root, root_sector);
    if (strncasecmp((char *)root_sector, "READY   TXT", 11) != 0)
        fail("drive status is '%.11s', not Ready; hold F4 to reset the drive", root_sector);
    // The bootloader scans for the file from entry 2, after the status file and volume label.
    for (int i = 64; i < SECTOR; i++) {
        if (root_sector[i]) fail("root directory has entries after the volume label; replug the device");
    }

    size_t padded_len = (len + SECTOR - 1) / SECTOR * SECTOR;
    size_t clusters = (padded_len + cluster_bytes - 1) / cluster_bytes;
    if (2 * (2 + clusters) > SECTOR) fail("file needs more than one FAT sector");

    uint8_t *entry = root_sector + 64;
    memcpy(entry, base, 7);
    memcpy(entry + 7, " BIN", 4);
    entry[11] = 0x20;
    put_le16(entry + 26, 2);
    put_le32(entry + 28, len);

    uint8_t *padded = malloc(padded_len);
    memcpy(padded, payload, len);
    memset(padded + len, 0xFF, padded_len - len);

    uint8_t fat[SECTOR] = {0xF8, 0xFF, 0xFF, 0xFF};
    for (size_t n = 2; n < 2 + clusters; n++) put_le16(fat + n * 2, n < 1 + clusters ? n + 1 : 0xFFFF);

    write_bytes(fd, root_sector, SECTOR, root);
    write_bytes(fd, padded, padded_len, data);
    for (unsigned i = 0; i < fat_count; i++) write_bytes(fd, fat, SECTOR, (off_t)(reserved + i * fat_sectors) * SECTOR);
    write_bytes(fd, root_sector, SECTOR, root);
    fsync(fd);
    free(padded);
    return root;
}

// When done, the bootloader renames the status file to Success, then drops off USB for about
// 2 s and comes back Ready. The drop only follows a finished write.
static void wait_for_success(int fd, off_t root) {
    uint8_t root_sector[SECTOR];
    // The drive can come back between two polls, so clear found_disk before the first.
    found_disk[0] = 0;
    for (int i = 0; i < 10 / POLL_S; i++) {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, POLL_S, false);
        if (pread(fd, root_sector, SECTOR, root) != SECTOR) {
            printf("The device dropped off USB, which means it finished. Waiting for it to return...\n");
            // Keep blocking the mount until the drive is back, so macOS writes nothing to it.
            if (!wait_for_drive(15)) fail("the device did not return within 15 s");
            return;
        }
        if (strncasecmp((char *)root_sector, "SUCCESS", 7) == 0) return;
        if (strncasecmp((char *)root_sector, "READY   TXT", 11) != 0)
            fail("status now '%.11s', not Success", root_sector);
    }
    fail("status still Ready after 10 s; the device did not take the file");
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s FILE   (A000014.bin, F0000C0.bin, ...)\n", argv[0]);
        return 1;
    }
    const char *path = argv[1];
    char base[8];
    update_name(path, base);
    size_t len;
    uint8_t *payload = read_file(path, &len);
    check_range(path, base, len);
    check_not_mounted();

    DASessionRef session = DASessionCreate(kCFAllocatorDefault);
    DARegisterDiskMountApprovalCallback(session, NULL, approve_mount, NULL);
    DASessionScheduleWithRunLoop(session, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);

    printf("Installing %s.BIN (%zu bytes).\n\n", base, len);
    printf("1. Unplug the device, and turn it off.\n");
    printf("2. Hold F1 and F4. Keep holding them, and hold the dial down too.\n");
    printf("3. When the firmware update screen shows, let go of all three.\n");
    printf("4. Plug the device into this Mac.\n\n");
    printf("Waiting for the device (up to %d seconds; Control-C to stop)...\n", WAIT_S);
    fflush(stdout);
    if (!wait_for_drive(WAIT_S))
        fail("no update drive appeared in %d seconds. Check that the device shows the firmware update screen, and that the cable carries data.", WAIT_S);

    char raw[80];
    snprintf(raw, sizeof raw, "/dev/r%s", found_disk);
    printf("Found the device at %s.\n\n", raw);
    printf("macOS now asks for your Mac login password, to allow writing to the device.\n");
    fflush(stdout);
    int fd = authopen_rw(raw);

    printf("Writing...\n");
    fflush(stdout);
    off_t root = write_update(fd, base, payload, len);
    wait_for_success(fd, root);
    printf("\nFinished. Unplug the device.\n");
    return 0;
}
