#!/usr/bin/env bash
set -euo pipefail

if [[ "${APPLY_DLL_PATCH:-false}" != "true" ]]; then
  echo "[apply-patch] APPLY_DLL_PATCH is not true — skipping."
  exit 0
fi

PATCH_SRC="/patches/assembly_valheim.dll"
TARGET="/opt/valheim/server/valheim_server_Data/Managed/assembly_valheim.dll"

if [[ ! -f "${PATCH_SRC}" ]]; then
  echo "[apply-patch] FATAL: patched DLL not found at ${PATCH_SRC}" >&2
  exit 1
fi

if [[ ! -f "${TARGET}" ]]; then
  echo "[apply-patch] FATAL: target DLL not found at ${TARGET}" >&2
  exit 1
fi

SRC_MD5=$(md5sum "${PATCH_SRC}" | cut -d' ' -f1)
DST_MD5=$(md5sum "${TARGET}"    | cut -d' ' -f1)

if [[ "${SRC_MD5}" == "${DST_MD5}" ]]; then
  echo "[apply-patch] OK (already patched): ${TARGET}"
  exit 0
fi

TMP_TARGET="${TARGET}.patching"
cp --preserve=timestamps "${PATCH_SRC}" "${TMP_TARGET}"
mv "${TMP_TARGET}" "${TARGET}"
chmod 644 "${TARGET}"

echo "[apply-patch] PATCHED: ${TARGET}"
echo "[apply-patch] Done."
