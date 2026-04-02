#!/usr/bin/sh
# Append one failure block to MATRIX_FAILURES.md (used by run-smp2-guest-matrix.sh).
# Usage: append-matrix-failure-note.sh <logdir> <probe> <qemu|oracle> <log_file>
set -eu
logdir="${1:?logdir}"
probe="${2:?probe}"
kind="${3:?kind qemu|oracle}"
log_file="${4:?log file}"
mkdir -p "$logdir"
report="$logdir/MATRIX_FAILURES.md"
if [ ! -f "$report" ]; then
  {
    echo "# SMP2 guest matrix failures"
    echo ""
    echo "生成：\`run-smp2-guest-matrix.sh\`（失败时追加）。处理步骤见 **\`docs/starryos-probes-matrix-failure-playbook.md\`**。"
    echo ""
  } >>"$report"
fi
{
  echo "## ${probe} (${kind})"
  echo ""
  echo "- **Log file**: \`$log_file\`"
  echo "- **Playbook**: \`docs/starryos-probes-matrix-failure-playbook.md\`"
  echo ""
  if [ "$kind" = oracle ]; then
    echo "### 建议下一步"
    echo ""
    echo "1. \`test-suit/starryos/scripts/extract-case-line.sh '$log_file'\`（单行）或 \`extract-case-lines.sh\` / \`diff-guest-cases.sh $probe '$log_file'\`（\`.cases\`）"
    echo "2. \`test-suit/starryos/scripts/run-diff-probes.sh verify-oracle $probe\`"
    echo "3. 对比 \`test-suit/starryos/probes/expected/${probe}.line\` 或 \`${probe}.cases\`"
    echo ""
  else
    echo "### 建议下一步"
    echo ""
    echo "1. 查看日志尾部：\`tail -80 '$log_file'\`"
    echo "2. 单探针：\`test-suit/starryos/scripts/run-starry-probe-qemu-smp2.sh $probe\`"
    echo ""
  fi
  echo "---"
  echo ""
} >>"$report"
