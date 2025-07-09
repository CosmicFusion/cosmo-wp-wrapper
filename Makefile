all:
	true

install:
	mkdir -p $(DESTDIR)/usr/bin/
	cargo fetch
	cargo build --release
	cp -vf target/release/cosmo-wp-wrapper $(DESTDIR)/usr/bin/
	chmod 755 $(DESTDIR)/usr/bin/cosmo-wp-wrapper
