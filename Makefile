compiler:
	mkdir -p 'src/compiler/bin' \
	&& wget 'https://github.com/matter-labs/zksolc-bin/raw/main/macosx-arm64/zksolc-macosx-arm64-v1.3.14' -O 'src/compiler/bin/zksolc' \
	&& chmod +x 'src/compiler/bin/zksolc' \
	&& brew install solidity \
	&& cp /opt/homebrew/bin/solc src/compiler/bin/solc

