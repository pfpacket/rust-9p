import getpass
import os
import shlex
import shutil
import socket
import subprocess
import sys
import logging


def error(message: str, exitstatus: int = 4) -> int:
    print("error:", message, file=sys.stderr)
    return exitstatus


def main() -> int:
    """Run the routine."""
    logging.basicConfig(level=logging.INFO)
    logger = logging.getLogger()

    hostname = socket.gethostname()
    try:
        source = sys.argv[1]
        target = sys.argv[2]
    except IndexError:
        relfile = os.path.relpath(__file__, os.getcwd())
        return error(
            f"""you did not specify the source or the target folders

usage:

    export PATH="/path/to/unpfs/binary:$PATH"
    {sys.executable} {relfile} <folder to export through unpfs> <mount point for the exported folder>

Note that this program will attempt to run the mount operation using sudo.
If sudo is not passwordless and you are not running as root, the mount
operation will fail.

To see real-time information from unpfs, export variable RUST_LOG=info
before executing this program.
""",
            os.EX_USAGE,
        )

    if not shutil.which("unpfs"):
        return error(
            "unpfs cannot be found in the system PATH",
            os.EX_USAGE,
        )

    f1_read, f1_write = os.pipe2(0)
    f2_read, f2_write = os.pipe2(0)

    stdin_for_read = os.fdopen(f1_read, "rb", buffering=0)
    stdout_for_write = os.fdopen(f2_write, "wb", buffering=0)
    stdin_for_write = os.fdopen(f1_write, "wb", buffering=0)
    stdout_for_read = os.fdopen(f2_read, "rb", buffering=0)

    # With the fingerprint, use it to invoke the RPC service that was
    # just authorized for this script.
    cmdline = ["unpfs", "fd!0!1", sys.argv[1]]
    env = dict(os.environ.items())
    env["RUST_LOG"] = "info"
    logger.info("Running %s", shlex.join(cmdline))
    subprocess.Popen(
        cmdline,
        stdin=stdin_for_read,
        stdout=stdout_for_write,
        bufsize=0,
        close_fds=True,
        env=env,
    )
    stdin_for_read.close()
    stdout_for_write.close()

    uid = os.getuid()
    gid = os.getgid()
    username = getpass.getuser()
    cmdline = [
        "/usr/bin/sudo",
        "/usr/bin/mount",
        "-t",
        "9p",
        "-o",
        "trans=fd,rfdno=%s,wfdno=%s,version=9p2000.L,dfltuid=%s,dfltgid=%s,uname=%s,aname=%s"
        % (0, 1, uid, gid, username, source),
        "unpfs://%s%s" % (hostname, source),
        target,
    ]
    logger.info("Running %s", shlex.join(cmdline))
    p2 = subprocess.Popen(
        cmdline, stdin=stdout_for_read, stdout=stdin_for_write, close_fds=True
    )
    stdout_for_read.close()
    stdin_for_write.close()

    return p2.wait()


if __name__ == "__main__":
    sys.exit(main())
