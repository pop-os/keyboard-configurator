prefix ?= /usr/local
libdir = $(prefix)/lib
includedir = $(prefix)/include

TARGET = debug
DEBUG ?= 0
ifeq ($(DEBUG),0)
	TARGET = release
	ARGS += --release
endif

VENDOR ?= 0
ifneq ($(VENDOR),0)
	ARGS += --frozen
endif

PACKAGE = pop_keyboard_backlight
PKGCONFIG = target/$(PACKAGE).pc
FFI = target/$(TARGET)/lib$(PACKAGE).so

all: $(BIN) $(PKGCONFIG)

clean:
	rm -rf target

distclean: clean
	rm -rf .cargo vendor vendor.tar

$(BIN): Cargo.toml Cargo.lock src/lib.rs vendor-check
	cargo build $(ARGS)

$(FFI): Cargo.toml Cargo.lock ffi/src/lib.rs vendor-check
	cargo build $(ARGS) --manifest-path ffi/Cargo.toml

install:
	install -Dm0644 target/$(TARGET)/lib$(PACKAGE).so "$(DESTDIR)$(libdir)/lib$(PACKAGE).so"
	install -Dm0644 $(PKGCONFIG) "$(DESTDIR)$(libdir)/pkgconfig/$(PACKAGE).pc"
	install -Dm0644 ffi/$(PACKAGE).h "$(DESTDIR)$(includedir)/$(PACKAGE).h"

$(PKGCONFIG): $(FFI) tools/src/pkgconfig.rs
	cargo run -p tools --bin pkgconfig $(DESKTOP_ARGS) -- \
		$(PACKAGE) $(libdir) $(includedir)

## Cargo Vendoring

vendor:
	rm .cargo -rf
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar cf vendor.tar vendor
	rm -rf vendor

vendor-check:
ifeq ($(VENDOR),1)
	rm vendor -rf && tar xf vendor.tar
endif
