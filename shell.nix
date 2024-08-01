{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
	nativeBuildInputs = with pkgs.buildPackages;
	[
        rustc
        cargo
        mold

        cargo-deny
        cargo-tarpaulin
        cargo-audit
        cargo-nextest
        clippy
        rustfmt
		openssl
		pkg-config


		sqlx-cli
		mpv
        xorg.libxcb
	];
}
