install:
	cargo build --release
	sudo install -m755 target/release/shesh /usr/bin/shesh
	sudo sh -c "grep -qx '/usr/bin/shesh' /etc/shells || echo '/usr/bin/shesh' >> /etc/shells"

uninstall:
	sudo rm -f /usr/bin/shesh
	sudo sh -c "sed -i '/^\/usr\/bin\/shesh$$/d' /etc/shells"

clean:
	cargo clean
