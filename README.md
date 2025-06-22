# CAT-Tester

## Overview

A tool in rust that test Common-Access-Token(CAT) implementation for fetching streaming data from a remote server.
I wrote this program as exercise while learning rust, so it may contain issues that I'm not aware of. As I need some extra features I had to modify the common-access-token crate and the modified version can be found at https://github.com/chrjoh/common-access-token. There is an extra fix to handle airplay explained bellow.

## Usage

```
cargo run -- --help # for help
```

The default parameters that are given will create a token that work with the online cat parser found at https://cta-token.net/

The url parameter is assumed to point to a endpoint that returns a manifest that follow this format(see bellow)
the segments under EXTiNF can be of both full url or a path. The code will for example do this

Fetch manifest from https://example.com/asset/5245.isml/5245-video=2499968.m3u8 and rewrite the url to
https://example.com/asset/5245.isml/hls/5245-video=5000000-455767831.m4s to fetch the same segment(the first found in the manifest file) max_iteration times with a sleep for four seconds(default time, can be changed) for each fetch.

To handle the case with airplay the airplay url is constructed by adding CAT token, with cookie refresh as a query
parameter named CAT. This extra fix need to be handled serverside to move the query value into a set-cookie in the response.

### Manifest format

```

#EXTM3U
#EXTM3U
#EXT-X-VERSION:6 ## Created with Unified Streaming Platform (version=1.13.0-29687)
#EXT-X-MEDIA-SEQUENCE:455767831
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-TARGETDURATION:6
#USP-X-TIMESTAMP-MAP:MPEGTS=8322344688,LOCAL=2025-06-17T08:21:08.040000Z
#EXT-X-MAP:URI="hls/5245-video=5000000.m4s"
#EXT-X-PROGRAM-DATE-TIME:2025-06-17T08:21:08.040000Z
#EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAdXBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAFUSEHBv5PWh/DrSr0mmaYuCK60aCGNhc3RsYWJzIihleUpoYzNObGRFbGtJam9pZEhadFpXUnBZUzB5TURRMk5UY3lNU0o5MgdkZWZhdWx0SPPGiZsG",KEYFORMAT="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed",KEYFORMATVERSIONS="1"
#EXT-X-KEY:METHOD=SAMPLE-AES,URI="data:text/plain;charset=UTF-16;base64,KgMAAAEAAQAgAzwAVwBSAE0ASABFAEEARABFAFIAIAB2AGUAcgBzAGkAbwBuAD0AIgA0AC4AMwAuADAALgAwACIAIAB4AG0AbABuAHMAPQAiAGgAdAB0AHAAOgAvAC8AcwBjAGgAZQBtAGEAcwAuAG0AaQBjAHIAbwBzAG8AZgB0AC4AYwBvAG0ALwBEAFIATQAvADIAMAAwADcALwAwADMALwBQAGwAYQB5AFIAZQBhAGQAeQBIAGUAYQBkAGUAcgAiAD4APABEAEEAVABBAD4APABQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsASQBEAFMAPgA8AEsASQBEACAAVgBBAEwAVQBFAD0AIgA5AGUAUgB2AGMAUAB5AGgAMABqAHEAdgBTAGEAWgBwAGkANABJAHIAcgBRAD0APQAiACAAQQBMAEcASQBEAD0AIgBBAEUAUwBDAEIAQwAiACAALwA+ADwALwBLAEkARABTAD4APAAvAFAAUgBPAFQARQBDAFQASQBOAEYATwA+ADwATABBAF8AVQBSAEwAPgBoAHQAdABwAHMAOgAvAC8AbABpAGMALgBkAHIAbQB0AG8AZABhAHkALgBjAG8AbQAvAGwAaQBjAGUAbgBzAGUALQBwAHIAbwB4AHkALQBoAGUAYQBkAGUAcgBhAHUAdABoAC8AZAByAG0AdABvAGQAYQB5AC8AUgBpAGcAaAB0AHMATQBhAG4AYQBnAGUAcgAuAGEAcwBtAHgAPAAvAEwAQQBfAFUAUgBMAD4APABMAFUASQBfAFUAUgBMAD4AaAB0AHQAcABzADoALwAvAHAAbABhAHkAcgBlAGEAZAB5AC0AdQBpAC4AZQB4AGEAbQBwAGwAZQAuAGMAbwBtADwALwBMAFUASQBfAFUAUgBMAD4APABEAEUAQwBSAFkAUABUAE8AUgBTAEUAVABVAFAAPgBPAE4ARABFAE0AQQBOAEQAPAAvAEQARQBDAFIAWQBQAFQATwBSAFMARQBUAFUAUAA+ADwALwBEAEEAVABBAD4APAAvAFcAUgBNAEgARQBBAEQARQBSAD4A",KEYFORMAT="com.microsoft.playready",KEYFORMATVERSIONS="1"
#EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://drmtoday?assetId=media-20465721&variantId&keyId=706fe4f5a1fc3ad2af49a6698b822bad",KEYFORMAT="com.apple.streamingkeydelivery",KEYFORMATVERSIONS="1"
#EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAn3Bzc2gAAAAAPV5tNZuaQei4Q908bnLELAAAAH97InZlcnNpb24iOiJWMS4wIiwia2lkcyI6WyJjRy9rOWFIOE90S3ZTYVpwaTRJcnJRPT0iXSwiY29udGVudElEIjoiZXlKaGMzTmxkRWxrSWpvaWRIWnRaV1JwWVMweU1EUTJOVGN5TVNKOSIsImVuc2NoZW1hIjoiY2JjcyJ9",IV=0xE105A618D09DC0CCFFCDBCCA711E6BD0,KEYFORMAT="urn:uuid:3d5e6d35-9b9a-41e8-b843-dd3c6e72c42c",KEYFORMATVERSIONS="1"
#EXTINF:3.84, no desc
hls/5245-video=5000000-455767831.m4s
#EXTINF:3.84, no desc
hls/5245-video=5000000-455767832.m4s

```
