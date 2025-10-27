package main

import _ "embed"

//go:embed embed_templates/vless_reality_grpc_tun.json
var TplVlessRealityGrpcTun []byte

//go:embed embed_templates/vless_reality_grpc_proxy.json
var TplVlessRealityGrpcProxy []byte
