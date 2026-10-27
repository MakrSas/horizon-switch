# libnxshim.a: the C glue the Switch build links (see nxshim.c). OUT is where it goes.
DEVKITPRO ?= /opt/devkitpro
OUT ?= .
CC := $(DEVKITPRO)/devkitA64/bin/aarch64-none-elf-gcc
AR := $(DEVKITPRO)/devkitA64/bin/aarch64-none-elf-ar
CFLAGS := -O2 -fno-omit-frame-pointer -march=armv8-a+crc+crypto -mtp=soft -fPIE -D__SWITCH__ -I$(DEVKITPRO)/libnx/include
$(OUT)/libnxshim.a: $(OUT)/nxshim.o
	$(AR) rcs $@ $^
$(OUT)/nxshim.o: nxshim.c | $(OUT)
	$(CC) $(CFLAGS) -c $< -o $@
$(OUT):
	mkdir -p $@
