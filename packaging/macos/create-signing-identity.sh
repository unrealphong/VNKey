#!/bin/sh
# Creates a self-signed "VNKey Local Signing" code signing identity in the
# login keychain. build-app.sh signs with it, so macOS keeps the Accessibility
# permission when the app is rebuilt.
set -eu

name="VNKey Local Signing"
if security find-identity -p codesigning | grep -q "\"$name\""; then
  echo "$name already exists."
  exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cat >"$work/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $name
[ext]
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
basicConstraints = critical, CA:false
EOF
/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -config "$work/cert.cnf" -keyout "$work/key.pem" -out "$work/cert.pem"
/usr/bin/openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/cert.pem" \
  -name "$name" -out "$work/identity.p12" -passout pass:vnkey
security import "$work/identity.p12" -k "$HOME/Library/Keychains/login.keychain-db" \
  -P vnkey -T /usr/bin/codesign
echo "Created $name."
