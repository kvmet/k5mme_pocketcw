// Installs HamCube update files through the device's update drive, from macOS, in order and in one
// update-mode session.
// build: clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o pocketcw-flash pocketcw-flash.c
//            -framework DiskArbitration -framework CoreFoundation
// usage: pocketcw-flash FILE...   (F000400.bin A000014.bin, ...)
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
#include <sys/select.h>
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
struct update {
    const char *path;
    char base[8];
    uint8_t *payload;
    size_t len;
};

// Why the current attempt stopped. Attempt steps set it through `stop` and return 0.
static char problem[512];

static int stop(const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    vsnprintf(problem, sizeof problem, fmt, args);
    va_end(args);
    return 0;
}

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
static int check_not_mounted(void) {
    DIR *volumes = opendir("/Volumes");
    if (!volumes) return 1;
    struct dirent *entry;
    int mounted = 0;
    while ((entry = readdir(volumes))) mounted |= is_drive_name(entry->d_name);
    closedir(volumes);
    return mounted ? stop("the update drive is mounted. Eject it in Finder, and unplug the device.") : 1;
}

// Waits for Return while the run loop runs, so the update drive stays unmounted meanwhile.
static void wait_for_enter(void) {
    for (;;) {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.2, true);
        fd_set in;
        FD_ZERO(&in);
        FD_SET(STDIN_FILENO, &in);
        struct timeval now = {0, 0};
        if (select(STDIN_FILENO + 1, &in, NULL, NULL, &now) <= 0) continue;
        int c;
        while ((c = getchar()) != '\n' && c != EOF) {}
        if (c == EOF) fail("input closed");
        return;
    }
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
    if (n <= 0 || !cmsg || cmsg->cmsg_level != SOL_SOCKET || cmsg->cmsg_type != SCM_RIGHTS) {
        stop("macOS did not allow writing to the device. Type your Mac login password when macOS asks.");
        return -1;
    }
    int fd;
    memcpy(&fd, CMSG_DATA(cmsg), sizeof fd);
    return fd;
}

static int read_sector(int fd, off_t offset, uint8_t *buf) {
    return pread(fd, buf, SECTOR, offset) == SECTOR || stop("cannot read the device: %s", strerror(errno));
}

static int write_bytes(int fd, const uint8_t *buf, size_t len, off_t offset) {
    return pwrite(fd, buf, len, offset) == (ssize_t)len || stop("cannot write the device: %s", strerror(errno));
}

// Sets `root` to the byte offset of the root directory.
static int write_update(int fd, const char *base, const uint8_t *payload, size_t len, off_t *root) {
    uint8_t boot[SECTOR];
    if (!read_sector(fd, 0, boot)) return 0;
    unsigned bytes_per_sector = le16(boot + 11), sectors_per_cluster = boot[13];
    unsigned reserved = le16(boot + 14), fat_count = boot[16], root_entries = le16(boot + 17);
    unsigned fat_sectors = le16(boot + 22);
    if (bytes_per_sector != SECTOR || memcmp(boot + 54, "FAT16   ", 8) != 0)
        return stop("unexpected boot sector (%u B sectors, not FAT16)", bytes_per_sector);
    *root = (off_t)(reserved + fat_count * fat_sectors) * SECTOR;
    off_t data = *root + root_entries * 32;
    size_t cluster_bytes = sectors_per_cluster * SECTOR;
    unsigned total_sectors = le16(boot + 19) ? le16(boot + 19) : le16(boot + 32) | (unsigned)le16(boot + 34) << 16;
    printf("Drive: %zu B clusters, %u sectors, files up to %zu B\n", cluster_bytes, total_sectors,
           (SECTOR / 2 - 2) * cluster_bytes);

    uint8_t root_sector[SECTOR];
    if (!read_sector(fd, *root, root_sector)) return 0;
    printf("Device status: %.11s\n", root_sector);
    if (strncasecmp((char *)root_sector, "READY   TXT", 11) != 0)
        return stop("the device status is '%.11s', not Ready. Hold F4 to reset the drive.", root_sector);
    // The bootloader scans for the file from entry 2, after the status file and volume label.
    for (int i = 64; i < SECTOR; i++) {
        if (root_sector[i]) return stop("the drive already holds a file. Hold F4 to reset the drive.");
    }

    size_t padded_len = (len + SECTOR - 1) / SECTOR * SECTOR;
    size_t clusters = (padded_len + cluster_bytes - 1) / cluster_bytes;
    if (2 * (2 + clusters) > SECTOR) return stop("the file needs more than one FAT sector");

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

    int ok = write_bytes(fd, root_sector, SECTOR, *root) && write_bytes(fd, padded, padded_len, data);
    for (unsigned i = 0; ok && i < fat_count; i++) ok = write_bytes(fd, fat, SECTOR, (off_t)(reserved + i * fat_sectors) * SECTOR);
    ok = ok && write_bytes(fd, root_sector, SECTOR, *root);
    fsync(fd);
    free(padded);
    return ok;
}

// When done, the bootloader renames the status file to Success for about 1 s, then drops off
// USB for about 2 s and comes back Ready. Only a Success read proves the write.
static int wait_for_success(int fd, off_t root) {
    uint8_t root_sector[SECTOR];
    for (int i = 0; i < 10 / POLL_S; i++) {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, POLL_S, false);
        if (pread(fd, root_sector, SECTOR, root) != SECTOR)
            return stop("the device dropped off USB before showing Success, so the write is not confirmed.");
        if (strncasecmp((char *)root_sector, "READY   TXT", 11) == 0) continue;
        printf("Device status: %.11s\n", root_sector);
        if (strncasecmp((char *)root_sector, "SUCCESS", 7) == 0) return 1;
        return stop("the device status is '%.11s', not Success.", root_sector);
    }
    return stop("the status stayed Ready for 10 s; the device did not take the file.");
}

// Writes one file to the drive in found_disk, and waits for Success.
static int write_file(const struct update *file) {
    char raw[80];
    snprintf(raw, sizeof raw, "/dev/r%s", found_disk);
    printf("Found the device at %s.\n\n", raw);
    printf("macOS can ask for your Mac login password, to allow writing to the device.\n");
    fflush(stdout);
    int fd = authopen_rw(raw);
    if (fd < 0) return 0;

    printf("Writing %s.BIN...\n", file->base);
    fflush(stdout);
    off_t root;
    int ok = write_update(fd, file->base, file->payload, file->len, &root) && wait_for_success(fd, root);
    close(fd);
    return ok;
}

// One install attempt, from the setup steps to Success on the last file. Every attempt starts at
// the first file, because the bootloader can fail an A write that no F write came before.
static int install(const struct update *files, int count) {
    if (!check_not_mounted()) return 0;
    printf("1. Unplug the device, and turn it off.\n");
    printf("2. Hold F1 and F4. Keep holding them, and hold the dial down too.\n");
    printf("3. When the firmware update screen shows, let go of all three.\n");
    printf("4. Plug the device into this Mac.\n\n");
    printf("Waiting for the device (up to %d seconds; Control-C to stop)...\n", WAIT_S);
    fflush(stdout);
    found_disk[0] = 0;
    if (!wait_for_drive(WAIT_S))
        return stop("no update drive appeared in %d seconds. Check that the device shows the firmware update screen, and that the cable carries data.", WAIT_S);

    for (int i = 0; i < count; i++) {
        if (i > 0) {
            printf("\nWaiting for the device to come back (up to 15 s)...\n");
            fflush(stdout);
            found_disk[0] = 0;
            if (!wait_for_drive(15)) return stop("the device did not come back after %s.BIN.", files[i - 1].base);
        }
        if (!write_file(&files[i])) return 0;
    }
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s FILE...   (F000400.bin A000014.bin, ...)\n", argv[0]);
        return 1;
    }
    int count = argc - 1;
    struct update *files = calloc(count, sizeof *files);
    for (int i = 0; i < count; i++) {
        files[i].path = argv[i + 1];
        update_name(files[i].path, files[i].base);
        files[i].payload = read_file(files[i].path, &files[i].len);
        check_range(files[i].path, files[i].base, files[i].len);
    }

    DASessionRef session = DASessionCreate(kCFAllocatorDefault);
    DARegisterDiskMountApprovalCallback(session, NULL, approve_mount, NULL);
    DASessionScheduleWithRunLoop(session, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);

    for (int i = 0; i < count; i++) printf("Installing %s.BIN (%zu bytes).\n", files[i].base, files[i].len);
    printf("\n");
    while (!install(files, count)) {
        printf("\nProblem: %s\n", problem);
        printf("Press Return to try again, or Control-C to stop.\n");
        fflush(stdout);
        wait_for_enter();
        printf("\n");
    }
    // The drive comes back Ready after Success. Blocking its mount until then keeps macOS from
    // writing its own files to it.
    printf("\nInstalled: the device showed Success. Waiting for it to come back (up to 15 s)...\n");
    fflush(stdout);
    found_disk[0] = 0;
    wait_for_drive(15);
    printf("Finished. Unplug the device.\n");
    return 0;
}
