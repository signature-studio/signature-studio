# authenticode-report: `tools/authenticode_check.py` の結果（決定 0117、仮の決定）

作成 2026-10-29（JST）。ツール: `tools/authenticode_check.py`（標準ライブラリのみ）。対象: `binaries/win64/FLEngine_x64.dll`（SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`。インストーラ `flstudio_win64_26.1.7.5653.exe` から `tools/extract_nsis2.py --write FLEngine_x64.dll` で取り出したものと同一であることを確認）と、ホスト exe `FL64.exe`（同じインストーラから `tools/extract_nsis2.py --write FL64.exe` で**読み取り専用に**取り出した。抽出先 `lift/out-fruity4/inst/`。製品ファイルなので git に入れない）。

**ok の意味**: 「PE の Authenticode ダイジェストが署名されたダイジェストと一致し、署名値が埋め込みの署名者証明書の公開鍵で検証できた」。**検査していない（`unverified = chain,revocation,time`）**: 証明書チェーンのルートまでの信頼（署名者証明書自身の署名も検証しない）、失効、時刻の信頼（有効期間は印字するだけ。タイムスタンプは有無を報告するだけで、その署名も時刻も検証しない）。実機の Windows が同じ結果を返すことを意味しない。

## 要約

| ファイル | (a) ダイジェスト | (b) 署名 | 署名者 CN | 発行者 CN | 有効期間 | タイムスタンプ |
|---|---|---|---|---|---|---|
| FLEngine_x64.dll | **ok** | **ok** | Image Line NV | DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1 | 2026-07-28T00:00:00Z .. 2027-10-21T23:59:59Z | RFC 3161、2026-09-29T14:31:53Z（未検証） |
| FL64.exe | **ok** | **ok** | Image Line NV | 同上 | 同上 | RFC 3161、2026-09-29T14:31:48Z（未検証） |

観察（実測）:

- 証明書の鎖: 署名者（`Image Line NV`）← 中間 CA（`DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1`、ファイルに埋め込み済み、有効期間 2021-04-29 .. 2036-04-28）← **`DigiCert Trusted Root G4`（ファイルに埋め込まれていない）**。ルートまでの信頼は検証していない。
- 署名者証明書の notBefore（**2026-07-28**）は、この環境の仮想時計（2021-01-01、決定 0018）より後。タイムスタンプの時刻（2026-09-29）は有効期間の内側。Learn は `WTD_LIFETIME_SIGNING_FLAG` が無ければ「タイムスタンプ付きの署名は永久に有効」と書く（エンジンの呼び出しは `dwProvFlags = 0`）。実機が 2021 年の時計で何を返すかは**未確認**。
- 同じインストーラの `FL64 (scaled).exe`（SHA-256 `0ab91f0f…e979`）も (a) ok、(b) ok、署名者・発行者・有効期間は同じ（ダイジェストは別: `2104b741…3b1f`）。表には載せていない（依頼は `FL64.exe` だけ）。
  ```
  FL64 (scaled).exe  sha256=0ab91f0f1e947ec0  (a)=ok (b)=ok  signer CN=Image Line NV  issuer CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1  2026-07-28T00:00:00Z..2027-10-21T23:59:59Z  timestamp=rfc3161@2026-09-29T14:31:46Z  unverified=chain,revocation,time
  ```
- 署名はどちらも 1 つ（ネストした署名なし）、SHA-256、署名アルゴリズム OID は rsaEncryption（`1.2.840.113549.1.1.1`）、RSA 3072 ビット。

## ツールの出力（`--report`）

Offline: digest of the PE and signature value against the embedded signer certificate. **Not checked: chain, revocation, time.**

### FLEngine_x64.dll

- path: `binaries/win64/FLEngine_x64.dll`
- size: 22227016 bytes, SHA-256 of the file: `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`
- certificate table: offset 22216704, size 10312; WIN_CERTIFICATE dwLength 10312, revision 0x0200, type 0x0002; PKCS#7 10304 bytes

#### signature: primary

- PE digest algorithm: sha256; digest computed from the file: `16a77cf976070f9d356a3f575451239a1452a839e45cafc95a9c2217f9ddd5d0`
- digest in SpcIndirectDataContent: `16a77cf976070f9d356a3f575451239a1452a839e45cafc95a9c2217f9ddd5d0`
- **(a) digest: ok** - the sha256 Authenticode digest of the file equals the digest in SpcIndirectDataContent
- **(b) signature: ok** - contentType and messageDigest match; the RSA signature verifies with the signer certificate's public key
- SignerInfo digestAlgorithm: sha256; signatureAlgorithm OID: 1.2.840.113549.1.1.1; embedded certificates: 2
- signer certificate subject: `CN=Image Line NV, O=Image Line NV, L=Gent, C=BE`
- signer certificate issuer: `CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1, O=DigiCert, Inc., C=US`
- serial number: `0afc3aad2fa7d5b5e9c267ed4a67eb12`
- validity: notBefore 2026-07-28T00:00:00Z, notAfter 2027-10-21T23:59:59Z (printed; not compared with any clock)
- RSA modulus: 3072 bits
- timestamp: rfc3161, signing time 2026-09-29T14:31:53Z (presence reported; the timestamp's own signature and time are NOT verified)
- nested signatures carried: 0

### FL64.exe

- path: `lift/out-fruity4/inst/FL64.exe`
- size: 597064 bytes, SHA-256 of the file: `4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65`
- certificate table: offset 586752, size 10312; WIN_CERTIFICATE dwLength 10312, revision 0x0200, type 0x0002; PKCS#7 10304 bytes

#### signature: primary

- PE digest algorithm: sha256; digest computed from the file: `2853174a524a4a9dadc8205b64ee02726a610673e50681913d2a744e0948070c`
- digest in SpcIndirectDataContent: `2853174a524a4a9dadc8205b64ee02726a610673e50681913d2a744e0948070c`
- **(a) digest: ok** - the sha256 Authenticode digest of the file equals the digest in SpcIndirectDataContent
- **(b) signature: ok** - contentType and messageDigest match; the RSA signature verifies with the signer certificate's public key
- SignerInfo digestAlgorithm: sha256; signatureAlgorithm OID: 1.2.840.113549.1.1.1; embedded certificates: 2
- signer certificate subject: `CN=Image Line NV, O=Image Line NV, L=Gent, C=BE`
- signer certificate issuer: `CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1, O=DigiCert, Inc., C=US`
- serial number: `0afc3aad2fa7d5b5e9c267ed4a67eb12`
- validity: notBefore 2026-07-28T00:00:00Z, notAfter 2027-10-21T23:59:59Z (printed; not compared with any clock)
- RSA modulus: 3072 bits
- timestamp: rfc3161, signing time 2026-09-29T14:31:48Z (presence reported; the timestamp's own signature and time are NOT verified)
- nested signatures carried: 0

## 独立な照合（実測。ツールの一部ではない）

ツールの結果を、別の実装で確かめた。**限界**: DER の切り出し（PKCS#7 の署名値と署名属性の位置）にはツールの読み取りを使った。検証そのもの（ダイジェスト、RSA）は別の実装。

1. **OpenSSL 3.5.5** `openssl pkcs7 -inform DER -print_certs -noout -text`（PKCS#7 は PE の証明書テーブルから切り出した）の抜粋。subject、issuer、有効期間、serial がツールの出力と一致する。

### FLEngine_x64.dll (`openssl pkcs7 -inform DER -print_certs -noout -text`、抜粋)

```
        Serial Number:
            0a:fc:3a:ad:2f:a7:d5:b5:e9:c2:67:ed:4a:67:eb:12
        Issuer: C=US, O=DigiCert, Inc., CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1
            Not Before: Jul 28 00:00:00 2026 GMT
            Not After : Oct 21 23:59:59 2027 GMT
        Subject: C=BE, L=Gent, O=Image Line NV, CN=Image Line NV
        Serial Number:
            08:ad:40:b2:60:d2:9c:4c:9f:5e:cd:a9:bd:93:ae:d9
        Issuer: C=US, O=DigiCert Inc, OU=www.digicert.com, CN=DigiCert Trusted Root G4
            Not Before: Apr 29 00:00:00 2021 GMT
            Not After : Apr 28 23:59:59 2036 GMT
        Subject: C=US, O=DigiCert, Inc., CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1
```

### FL64.exe (`openssl pkcs7 -inform DER -print_certs -noout -text`、抜粋)

```
        Serial Number:
            0a:fc:3a:ad:2f:a7:d5:b5:e9:c2:67:ed:4a:67:eb:12
        Issuer: C=US, O=DigiCert, Inc., CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1
            Not Before: Jul 28 00:00:00 2026 GMT
            Not After : Oct 21 23:59:59 2027 GMT
        Subject: C=BE, L=Gent, O=Image Line NV, CN=Image Line NV
        Serial Number:
            08:ad:40:b2:60:d2:9c:4c:9f:5e:cd:a9:bd:93:ae:d9
        Issuer: C=US, O=DigiCert Inc, OU=www.digicert.com, CN=DigiCert Trusted Root G4
            Not Before: Apr 29 00:00:00 2021 GMT
            Not After : Apr 28 23:59:59 2036 GMT
        Subject: C=US, O=DigiCert, Inc., CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1
```
2. **`pefile` 2024.8.26 + `hashlib`**（`/usr/bin/python3` 3.14.4。見出しのオフセットは `pefile`、ダイジェストは別に書いたマスク）、**`cryptography` 46.0.5**（`PKCS1v15` の RSA 検証。負の対照として署名属性を 1 ビット変えて `InvalidSignature` を確認）。スクリプトは `xcheck-independent.py`。

```
../../binaries/win64/FLEngine_x64.dll digest equal: True 16a77cf976070f9d messageDigest attr ok: True RSA verify (cryptography): ok subject: CN=Image Line NV,O=Image Line NV,L=Gent,C=BE
  negative control (signedAttrs flipped): rejected by cryptography: InvalidSignature
inst/FL64.exe digest equal: True 2853174a524a4a9d messageDigest attr ok: True RSA verify (cryptography): ok subject: CN=Image Line NV,O=Image Line NV,L=Gent,C=BE
  negative control (signedAttrs flipped): rejected by cryptography: InvalidSignature
```

## 再現

```
cd <repo root>
python3 tools/extract_nsis2.py "<installer>" --write FL64.exe <dir>/FL64.exe        # 読み取り専用（出力先は存在しないこと）
python3 tools/authenticode_check.py --tsv lift/harness/seeds/authenticode-claims.tsv --report <report>.md binaries/win64/FLEngine_x64.dll <dir>/FL64.exe
```

出力は決定的（時刻や乱数を含まない）。テスト `the_committed_claims_table_is_what_the_tool_writes_for_the_real_files` が、リポジトリの表がツールの現在の出力と一致することを固定している（FL64.exe は環境変数 `FL64_EXE` があるとき）。
