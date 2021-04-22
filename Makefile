prefix ?= /usr/local
bindir = $(prefix)/bin
libdir = $(prefix)/lib
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

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

PACKAGE = system76_keyboard_configurator
APPID = "com.system76.keyboardconfigurator"
PKGCONFIG = $(PACKAGE).pc
BIN = system76-keyboard-configurator
FFI = lib$(PACKAGE).so
APPDATA = $(APPID).appdata.xml
DESKTOP = $(APPID).desktop

all: $(BIN) $(PKGCONFIG)

clean:
	rm -rf target

distclean: clean
	rm -rf .cargo vendor vendor.tar

$(BIN): Cargo.toml Cargo.lock src/main.rs vendor-check
	cargo build $(ARGS)

$(FFI): Cargo.toml Cargo.lock ffi/src/lib.rs vendor-check
	cargo build $(ARGS) --manifest-path ffi/Cargo.toml

install:
	install -Dm0755 target/$(TARGET)/$(BIN) $(DESTDIR)$(bindir)/$(BIN)
	install -Dm0644 target/$(TARGET)/$(FFI) "$(DESTDIR)$(libdir)/$(FFI)"
	install -Dm0644 target/$(PKGCONFIG) "$(DESTDIR)$(libdir)/pkgconfig/$(PKGCONFIG)"
	install -Dm0644 ffi/$(PACKAGE).h "$(DESTDIR)$(includedir)/$(PACKAGE).h"
	install -Dm0644 "linux/$(DESKTOP)" "$(DESTDIR)$(datadir)/applications/$(DESKTOP)"
	install -Dm0644 "linux/$(APPDATA)" "$(DESTDIR)$(datadir)/metainfo/$(APPDATA)"

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
