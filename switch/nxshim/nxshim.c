// libc functions Rust std calls on target_os = "horizon" that devkitA64 newlib / libnx lack.
#include <switch.h>
#include <errno.h>
#include <sys/types.h>
#include <unistd.h>
#include <sys/stat.h>
#include <errno.h>

ssize_t getrandom(void *buf, size_t len, unsigned int flags) {
    (void)flags;
    randomGet(buf, len);
    return (ssize_t)len;
}

long sysconf(int name) {
    switch (name) {
    case _SC_PAGESIZE: return 0x1000;
    case _SC_NPROCESSORS_CONF:
    case _SC_NPROCESSORS_ONLN: return 3; // applications run on cores 0-2
    case _SC_THREAD_STACK_MIN: return 0x4000;
    case _SC_GETPW_R_SIZE_MAX: return 1024;
    default: errno = EINVAL; return -1;
    }
}

// No dynamic loader on the Switch: code that probes for optional libraries (the Wayland EGL
// library, libloading) gets a clean failure.
void *dlopen(const char *file, int mode) { (void)file; (void)mode; return 0; }
void *dlsym(void *handle, const char *name) { (void)handle; (void)name; return 0; }
int dlclose(void *handle) { (void)handle; return 0; }
char *dlerror(void) { return (char *)"the Switch has no dynamic loader"; }

// fatalThrow, wrapped (-Wl,--wrap=fatalThrow): before the system's fatal screen, write the result
// and the return addresses (frame-pointer walk) to the game's folder, with this function's own
// address so they can be matched against the ELF.
#include <stdio.h>
#include <stdint.h>
void __real_fatalThrow(Result err) NX_NORETURN;
void __wrap_fatalThrow(Result err) {
    svcOutputDebugString("horizon fatal", 13);
    FILE *f = fopen("sdmc:/switch/horizon/fatal.txt", "w");
    if (f) {
        fprintf(f, "result 0x%x (%04u-%04u)\nanchor __wrap_fatalThrow %p\n", err,
                2000 + R_MODULE(err), R_DESCRIPTION(err), (void *)&__wrap_fatalThrow);
        fprintf(f, "ra %p\n", __builtin_return_address(0));
        uintptr_t *fp = (uintptr_t *)__builtin_frame_address(0);
        for (int i = 0; i < 32 && fp && ((uintptr_t)fp & 7) == 0; i++) {
            uintptr_t lr = fp[1];
            if (!lr) break;
            fprintf(f, "fp %p lr %p\n", (void *)fp, (void *)lr);
            fp = (uintptr_t *)fp[0];
        }
        fclose(f);
    }
    __real_fatalThrow(err);
}

// diagAbortWithResult, wrapped (-Wl,--wrap=diagAbortWithResult): libnx's own aborts (a service
// used before its initialization, ...) leave the same trace as fatalThrow above.
void __real_diagAbortWithResult(Result res) NX_NORETURN;
void __wrap_diagAbortWithResult(Result res) {
    FILE *f = fopen("sdmc:/switch/horizon/fatal.txt", "w");
    if (f) {
        fprintf(f, "diagAbort result 0x%x (%04u-%04u)\nanchor __wrap_diagAbortWithResult %p\n", res,
                2000 + R_MODULE(res), R_DESCRIPTION(res), (void *)&__wrap_diagAbortWithResult);
        fprintf(f, "ra %p\n", __builtin_return_address(0));
        uintptr_t *fp = (uintptr_t *)__builtin_frame_address(0);
        for (int i = 0; i < 32 && fp && ((uintptr_t)fp & 7) == 0; i++) {
            uintptr_t lr = fp[1];
            if (!lr) break;
            fprintf(f, "fp %p lr %p\n", (void *)fp, (void *)lr);
            fp = (uintptr_t *)fp[0];
        }
        fclose(f);
    }
    __real_diagAbortWithResult(res);
}

// abort, wrapped (-Wl,--wrap=abort): Rust's panic=abort ends here. newlib's abort leaves through
// libnx's exit, which closes hid while other threads still read it (a misleading 2345-0008).
// Record the stack, then end the process without the exit handlers.
void __real_abort(void) NX_NORETURN;
void __wrap_abort(void) {
    FILE *f = fopen("sdmc:/switch/horizon/fatal.txt", "w");
    if (f) {
        fprintf(f, "abort\nanchor __wrap_abort %p\nra %p\n", (void *)&__wrap_abort,
                __builtin_return_address(0));
        uintptr_t *fp = (uintptr_t *)__builtin_frame_address(0);
        for (int i = 0; i < 48 && fp && ((uintptr_t)fp & 7) == 0; i++) {
            uintptr_t lr = fp[1];
            if (!lr) break;
            fprintf(f, "fp %p lr %p\n", (void *)fp, (void *)lr);
            fp = (uintptr_t *)fp[0];
        }
        fclose(f);
    }
    svcExitProcess();
    __real_abort();
}

// Rust's fatal runtime messages ("memory allocation of N bytes failed", panics before the log
// starts) go to stderr; on the Switch it goes nowhere, so it is kept in the game's folder.
__attribute__((constructor)) static void nxshim_keep_stderr(void) {
    mkdir("sdmc:/switch/horizon", 0777);
    if (freopen("sdmc:/switch/horizon/stderr.txt", "w", stderr))
        setvbuf(stderr, NULL, _IONBF, 0);
}

// The memory this process may use and has used (svcGetInfo TotalMemorySize/UsedMemorySize).
void nxshim_memory(uint64_t *total, uint64_t *used) {
    *total = 0;
    *used = 0;
    svcGetInfo(total, InfoType_TotalMemorySize, CUR_PROCESS_HANDLE, 0);
    svcGetInfo(used, InfoType_UsedMemorySize, CUR_PROCESS_HANDLE, 0);
}

// pthread keys, wrapped (-Wl,--wrap=pthread_key_*, pthread_[gs]etspecific): libnx keeps keys in
// the thread's TLS slots, about 27 of them, and Rust gives every thread_local! its own key.
// These keys live in a per-thread table held in one real libnx slot.
#include <pthread.h>
#include <stdlib.h>
#include <errno.h>
#define NX_KEYS 1024
int __real_pthread_key_create(pthread_key_t *key, void (*dtor)(void *));
void *__real_pthread_getspecific(pthread_key_t key);
int __real_pthread_setspecific(pthread_key_t key, const void *value);
static pthread_key_t nx_slot;
static pthread_once_t nx_once = PTHREAD_ONCE_INIT;
static void (*nx_dtors[NX_KEYS])(void *);
static unsigned char nx_used[NX_KEYS];
static unsigned nx_next;
static pthread_mutex_t nx_lock = PTHREAD_MUTEX_INITIALIZER;

static void nx_run_dtors(void *table) {
    void **t = table;
    // Destructors may use keys again: the table stays this thread's while they run.
    __real_pthread_setspecific(nx_slot, t);
    for (int round = 0; round < 4; round++) {
        int again = 0;
        for (unsigned k = 0; k < NX_KEYS; k++) {
            void *v = t[k];
            if (v && nx_dtors[k]) {
                t[k] = 0;
                nx_dtors[k](v);
                again = 1;
            }
        }
        if (!again) break;
    }
    __real_pthread_setspecific(nx_slot, 0);
    free(t);
}

static void nx_init(void) { __real_pthread_key_create(&nx_slot, nx_run_dtors); }

static void **nx_table(int create) {
    pthread_once(&nx_once, nx_init);
    void **t = __real_pthread_getspecific(nx_slot);
    if (!t && create) {
        t = calloc(NX_KEYS, sizeof(void *));
        if (t) __real_pthread_setspecific(nx_slot, t);
    }
    return t;
}

int __wrap_pthread_key_create(pthread_key_t *key, void (*dtor)(void *)) {
    pthread_once(&nx_once, nx_init);
    pthread_mutex_lock(&nx_lock);
    unsigned k = nx_next;
    while (k < NX_KEYS && nx_used[k]) k++;
    if (k >= NX_KEYS) {
        for (k = 0; k < NX_KEYS && nx_used[k]; k++) {}
    }
    if (k >= NX_KEYS) {
        pthread_mutex_unlock(&nx_lock);
        return EAGAIN;
    }
    nx_used[k] = 1;
    nx_dtors[k] = dtor;
    nx_next = k + 1;
    pthread_mutex_unlock(&nx_lock);
    *key = (pthread_key_t)k;
    return 0;
}

int __wrap_pthread_key_delete(pthread_key_t key) {
    if ((unsigned)key >= NX_KEYS) return EINVAL;
    pthread_mutex_lock(&nx_lock);
    nx_used[key] = 0;
    nx_dtors[key] = 0;
    pthread_mutex_unlock(&nx_lock);
    return 0;
}

void *__wrap_pthread_getspecific(pthread_key_t key) {
    if ((unsigned)key >= NX_KEYS) return 0;
    void **t = nx_table(0);
    return t ? t[key] : 0;
}

int __wrap_pthread_setspecific(pthread_key_t key, const void *value) {
    if ((unsigned)key >= NX_KEYS) return EINVAL;
    void **t = nx_table(1);
    if (!t) return ENOMEM;
    t[key] = (void *)value;
    return 0;
}

// threadCreate, wrapped (-Wl,--wrap=threadCreate): libnx starts every thread on the process's
// default core unless told otherwise, so the game's workers (chunk generation, meshing, loading)
// all shared core 0. Threads with no core of their own go round cores 1-3, leaving core 0 to the
// render thread; core 3 (the system's) is granted by the NSP's npdm. Where it is not, the thread
// starts on the default core as before.
//
// Every thread also starts in thread_start, which sets TPIDR_EL0: NVK carries Rust code built for
// Linux (its shader compiler, NAK), whose thread-locals are read from TPIDR_EL0, while libnx keeps
// the thread pointer in ThreadVars.tls_tp (offset 0x1f8 of the TLS area) and leaves the register 0.
static void set_thread_pointer(void) {
    uint64_t tp;
    __asm__ volatile("mrs %0, tpidrro_el0\n\t"
                     "ldr %0, [%0, #0x1f8]\n\t"
                     "msr tpidr_el0, %0"
                     : "=r"(tp));
}

// libnx calls this weak hook once its services are up, before constructors and main.
void userAppInit(void) {
    set_thread_pointer();
}

typedef struct {
    ThreadFunc entry;
    void *arg;
} ThreadStart;

static void thread_start(void *p) {
    set_thread_pointer();
    ThreadStart start = *(ThreadStart *)p;
    free(p);
    start.entry(start.arg);
}

Result __real_threadCreate(Thread *t, ThreadFunc entry, void *arg, void *stack_mem, size_t stack_sz,
                           int prio, int cpuid);
Result __wrap_threadCreate(Thread *t, ThreadFunc entry, void *arg, void *stack_mem, size_t stack_sz,
                           int prio, int cpuid) {
    static unsigned next;
    ThreadStart *start = malloc(sizeof *start);
    if (!start)
        return MAKERESULT(Module_Libnx, LibnxError_OutOfMemory);
    start->entry = entry;
    start->arg = arg;
    Result rc = -1;
    if (cpuid == -2) {
        static const int cores[3] = {1, 2, 0};
        int core = cores[__atomic_fetch_add(&next, 1, __ATOMIC_RELAXED) % 3];
        rc = __real_threadCreate(t, thread_start, start, stack_mem, stack_sz, prio, core);
    }
    if (R_FAILED(rc))
        rc = __real_threadCreate(t, thread_start, start, stack_mem, stack_sz, prio, cpuid);
    if (R_FAILED(rc))
        free(start);
    return rc;
}

// rename, wrapped (-Wl,--wrap=rename): POSIX replaces an existing target, the Switch's file system
// refuses (EEXIST). Saves write a temporary file and rename it over the old one.
int __real_rename(const char *from, const char *to);
int __wrap_rename(const char *from, const char *to) {
    int rc = __real_rename(from, to);
    if (rc != 0 && (errno == EEXIST || errno == EACCES || errno == EPERM)) {
        struct stat st;
        if (stat(to, &st) == 0 && !S_ISDIR(st.st_mode) && unlink(to) == 0)
            rc = __real_rename(from, to);
    }
    return rc;
}

// The prepared resources as one RomFS image on the SD card, mounted as `name:/`.
int nxshim_mount_romfs(const char *path, const char *name) {
    return R_SUCCEEDED(romfsMountFromFsdev(path, 0, name)) ? 0 : -1;
}

// The Switch port quits at once: joining every worker on the way out of the title screen hangs.
void nxshim_exit_now(void) {
    fflush(NULL);
    svcExitProcess();
}

// The cores this process may use (its kernel core mask): 4 when the npdm grants core 3.
size_t nxshim_cpu_count(void) {
    u64 mask = 0;
    if (R_FAILED(svcGetInfo(&mask, InfoType_CoreMask, CUR_PROCESS_HANDLE, 0)))
        return 3;
    return (size_t)__builtin_popcountll(mask & 0xF);
}

// HD Rumble for player 1: both Joy-Con (or the Pro Controller's two motors). The devices are
// opened once per controller style; a style libnx refuses is not retried.
void nxshim_rumble(bool handheld, u32 styles, float low, float high) {
    static HidVibrationDeviceHandle handles[2];
    static u32 opened_style;
    static Result opened_rc;
    HidNpadIdType id = handheld ? HidNpadIdType_Handheld : HidNpadIdType_No1;
    u32 style = handheld                               ? HidNpadStyleTag_NpadHandheld
                : (styles & HidNpadStyleTag_NpadFullKey) ? HidNpadStyleTag_NpadFullKey
                : (styles & HidNpadStyleTag_NpadJoyDual) ? HidNpadStyleTag_NpadJoyDual
                : (styles & HidNpadStyleTag_NpadJoyLeft) ? HidNpadStyleTag_NpadJoyLeft
                                                         : HidNpadStyleTag_NpadJoyRight;
    u32 key = style | (handheld ? 0x80000000u : 0);
    if (opened_style != key) {
        opened_style = key;
        opened_rc = hidInitializeVibrationDevices(handles, 2, id, (HidNpadStyleTag)style);
    }
    if (R_FAILED(opened_rc))
        return;
    HidVibrationValue v = {
        .amp_low = low, .freq_low = 160.0f, .amp_high = high, .freq_high = 320.0f,
    };
    HidVibrationValue values[2] = {v, v};
    bool single = style == HidNpadStyleTag_NpadJoyLeft || style == HidNpadStyleTag_NpadJoyRight;
    hidSendVibrationValues(handles, values, single ? 1 : 2);
}
