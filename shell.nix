{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
	nativeBuildInputs = with pkgs.buildPackages;
	[
		latest.rustChannels.stable.rust
		openssl
		pkg-config
		sqlx-cli
		mpv
		cargo-deny
        xorg.libxcb
	];
}

