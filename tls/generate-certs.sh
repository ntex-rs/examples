#!/bin/sh
set -eu
umask 077

out="$(dirname "$0")/certs"
if [ -e "$out" ]; then
    echo "$out already exists; move it aside before generating new identities." >&2
    exit 1
fi
mkdir "$out"

make_ca() {
    name="$1"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$out/$name.key"
    openssl req -new -x509 -sha256 -days 7 -key "$out/$name.key" \
        -out "$out/$name.pem" -subj "/CN=ntex example $name" \
        -addext "basicConstraints=critical,CA:TRUE" \
        -addext "keyUsage=critical,keyCertSign,cRLSign"
}

make_identity() {
    name="$1"
    ca="$2"
    usage="$3"
    san="$4"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$out/$name.key"
    openssl req -new -sha256 -key "$out/$name.key" \
        -out "$out/$name.csr" -subj "/CN=$name"
    printf '%s\n' "basicConstraints=critical,CA:FALSE" \
        "keyUsage=critical,digitalSignature,keyEncipherment" \
        "extendedKeyUsage=$usage" "subjectAltName=$san" > "$out/$name.ext"
    openssl x509 -req -sha256 -days 7 -in "$out/$name.csr" \
        -CA "$out/$ca.pem" -CAkey "$out/$ca.key" -CAcreateserial \
        -extfile "$out/$name.ext" -out "$out/$name.pem"
}

make_ca ca
make_ca untrusted-ca
make_identity server ca serverAuth "DNS:localhost,IP:127.0.0.1,IP:::1"
make_identity client ca clientAuth "URI:spiffe://ntex.example/client"
make_identity other-client ca clientAuth "URI:spiffe://ntex.example/other"
make_identity untrusted-client untrusted-ca clientAuth "URI:spiffe://ntex.example/client"

openssl x509 -in "$out/client.pem" -outform DER -out "$out/client.der"
openssl pkcs12 -export -out "$out/client.p12" -inkey "$out/client.key" \
    -in "$out/client.pem" -certfile "$out/ca.pem" -passout pass:
echo "Created local example identities in $out (valid for seven days)."
