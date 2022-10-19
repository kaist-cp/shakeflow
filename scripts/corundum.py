#! /usr/bin/python3

import subprocess
import pathlib
import argparse
import string
from time import sleep
import re
from datetime import datetime
import os

# ---------------------- GLOBAL CONSTANTS ----------------------

shakeflow_dir = pathlib.Path(__file__).absolute().parent.parent
corundum_dir = shakeflow_dir / 'corundum'

f00_pcie_map = {
    'f01': 'enp130s0f0',  # Slot 1
    'f03': 'enp69s0f0',   # Slot 4
    'f04': 'enp1s0f0',    # Slot 5
    'f05': 'enp194s0f0',  # Slot 6
    'f06': 'enp129s0f0',  # Slot 7
    'f07': 'enp193s0f0',  # Slot 3
}

# Modules that have their own testbench, and are also used for test_fpga_core.
# If a module has inappropriate module parameters for test_fpga_core,
# then include its modified module in `fpga_tb_module_pairs` below to overwrite it.
tb_modules = [
  "fpga/common/rtl/cmac_pad",
  "fpga/common/rtl/rx_checksum",
  "fpga/common/rtl/rx_hash",
  "fpga/common/rtl/tx_checksum",
  "fpga/common/rtl/queue_manager",
  "fpga/common/rtl/cpl_queue_manager",
]

# Modules that are only used for test_fpga_core.
# The first element indicates the name of the ShakeFlow module,
# and the second element indicates the path of the module in Corundum.
fpga_tb_module_pairs = [
    ('rx_checksum_512', 'fpga/common/rtl/rx_checksum'),
    ('rx_hash_512', 'fpga/common/rtl/rx_hash'),
    ('tx_checksum_512', 'fpga/common/rtl/tx_checksum'),
    ('event_mux', 'fpga/common/rtl/event_mux'),
    ('desc_fetch', 'fpga/common/rtl/desc_fetch'),
    ('tx_queue_manager', 'fpga/common/rtl/tx_queue_manager'),
    ('rx_queue_manager', 'fpga/common/rtl/rx_queue_manager'),
    ('event_cpl_queue_manager', 'fpga/common/rtl/event_cpl_queue_manager'),
    ('tx_cpl_queue_manager', 'fpga/common/rtl/tx_cpl_queue_manager'),
    ('rx_cpl_queue_manager', 'fpga/common/rtl/rx_cpl_queue_manager'),
    ('cpl_op_mux_mqnic_port', 'fpga/common/rtl/cpl_op_mux_mqnic_port'),
    ('cpl_op_mux_mqnic_interface', 'fpga/common/rtl/cpl_op_mux_mqnic_interface'),
    ('desc_op_mux', 'fpga/common/rtl/desc_op_mux'),
    ('cpl_write', 'fpga/common/rtl/cpl_write'),
    ('tx_engine', 'fpga/common/rtl/tx_engine'),
    ('rx_engine', 'fpga/common/rtl/rx_engine'),
    ('tx_scheduler_rr', 'fpga/common/rtl/tx_scheduler_rr'),
]

# Modules that are only used for bitstream generation.
# The first element indicates the name of the ShakeFlow module,
# and the second element indicates the path of the module in Corundum.
bitstream_gen_module_pairs = [
    ('event_cpl_queue_manager_bitstream', 'fpga/common/rtl/event_cpl_queue_manager'),
    ('tx_cpl_queue_manager_bitstream', 'fpga/common/rtl/tx_cpl_queue_manager'),
    ('rx_cpl_queue_manager_bitstream', 'fpga/common/rtl/rx_cpl_queue_manager'),
    ('tx_queue_manager_bitstream', 'fpga/common/rtl/tx_queue_manager'),
    ('rx_queue_manager_bitstream', 'fpga/common/rtl/rx_queue_manager'),
]

# Modules that are only used for test_fpga_core, and don't have `_inner` module file.
# The first element indicates the name of the ShakeFlow module,
# and the second element indicates the path of the module in Corundum.
fpga_tb_module_pairs_without_inner = [
    ('mqnic_interface', 'fpga/common/rtl/mqnic_interface'),
    ('mqnic_port', 'fpga/common/rtl/mqnic_port'),
]

# Lists which modules are used in place of original Corundum modules.
# Used for `program_per_module`.
module_replacements = {
    'queue_manager': ['tx_queue_manager', 'rx_queue_manager'],
    'cpl_queue_manager': ['tx_cpl_queue_manager', 'rx_cpl_queue_manager', 'event_cpl_queue_manager'],
    'cpl_op_mux': ['cpl_op_mux_mqnic_port', 'cpl_op_mux_mqnic_interface'],
}

# all_modules: List of all modules that have been ported.
all_modules = set()
replacement_modules = set()
x = list(map(lambda p: p.split('/')[-1], tb_modules))
all_modules.update(x)
for l in module_replacements.values():
    replacement_modules.update(l)
x = list(map(lambda p: p[1].split('/')[-1], fpga_tb_module_pairs))
x = list(filter(lambda p: p not in replacement_modules, x))
all_modules.update(x)
all_modules.update(module_replacements.keys())

help = """
Usage:
./scripts/corundum.py test_cocotb [--token CI_JOB_TOKEN] [--tb TB]
  If `token` is not present, test cocotb.
  If it is present, test CI.
  If `tb` is present, only test that module's tb. (ex. `fpga_core`)

./scripts/corundum.py program [--machine MACHINE] \\
  [--tb_exclude EXCLUDED_TBS] [--bit BITSTREAM_PATH] (e.g. f01)
  Generate bitstream. 
  If `machine` is specified, also upload and program to designated machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `tb_exclude` is specified, exclude mentioned modules when generating.
  `tb_exclude` is a bit-separated string.
  If `bit` is specified, program the given bit file to the machine instead of generating it.

./scripts/corundum.py bench --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  [--name NAME]
  Run iperf test, assuming bitstream is programmed to the machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If `name` is given, it is appended to the filename of the output CSV files.

./scripts/corundum.py bench_one --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01) \\
  [--mtu MTU] [--duplex DUPLEX] [--p PARALLEL_CONNECTIONS]
  Run one run of iperf test, assuming bitstream is programmed to the machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If experiment conditions are not given, they are defaulted to mtu 9000, half duplex, -p 5.

./scripts/corundum.py setup --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  Setup both machines so that Corundum is active.
  Specifically, this command performs `insmod` on Corundum, sets up both machines' IP addresses,
  sets up connection, and sets MTU to 9000.
  If `server_machine` is not given, it is defaulted to f00.

./scripts/corundum.py setup_nginx --machine $MACHINE (e.g. f01)
  Set up the machine for nginx experiment. ** This command must be run on both server and client machine. **
  Specifically, this command installs wrk, and sets up empty files for the server to read and sent to the client.
  WARNING: This command takes very long, about 4 hours.

./scripts/corundum.py bench_nic --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  [--name NAME]
  Run iperf test, assuming both machines have NICs attached.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If `name` is given, it is appended to the filename of the output CSV files.

./scripts/corundum.py util [--corundum_path CORUNDUM_PATH]
  Parse the utilization report that was created during `program` in given corundum_path.
  If `corundum_path` is not given, default to `$SHAKEFLOW_DIR/corundum` .

./scripts/corundum.py program_per_module [--tb $TB]
  Generate bitstreams only with ONE SHAKEFLOW MODULE AT A TIME.
  If tb (a comma-separated list of modules) is given, iterate through that list.
  If not, iterate through the entire list of modules.
  The resulting Corundum folders are located in ./corundum-\{module_name\}.

./scripts/corundum.py port_info
  Return TeX-formatted tabular table that will be used for paper.
  The columns respectively denote (module, Corundum LOC, ShakeFlow LOC, ShakeFlow (codegened) LOC).

./scripts/corundum.py fio --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  [--name NAME]
  Run fio test, assuming bitstream is programmed to the machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If `name` is given, it is appended to the filename of the output CSV files.

./scripts/corundum.py nginx_wrk --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  [--name NAME]
  Run nginx test, assuming bitstream is programmed to the machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If `name` is given, it is appended to the filename of the output CSV files.

./scripts/corundum.py nginx_scale --machine $MACHINE [--server_machine SERVER_MACHINE] (e.g. f01)
  [--name NAME]
  Run nginx scalability test, assuming bitstream is programmed to the machine.
  Assumes that f00~f07 is registered on your ssh config.
  If `server_machine` is not given, it is defaulted to f00.
  If `name` is given, it is appended to the filename of the output CSV files.
"""

wait_time = 2
sleep_time = 4
bench_time = 10

# -------------------- GLOBAL CONSTANTS END --------------------


# -------------------------- FUNCTIONS -------------------------

# Run the script, exit on error.
def run(args):
    subprocess.run(args, check=True)


# Get (path, name) tuple of given filepath string.
def get_path_and_name(path):
    module_path = pathlib.Path(path)
    dirname = module_path.parent
    basename = str(module_path.name)
    return (dirname, basename)


# Clone corundum.
# `token` is either `str` or `None`.
def setup_corundum(token):
    if corundum_dir.is_dir():
        if any(corundum_dir.iterdir()):
            # corundum_dir is not empty
            run(['git', '-C', corundum_dir, 'fetch', 'origin'])
            run(['git', '-C', corundum_dir, 'reset', '--hard', '45b7e35'])
        else:
            # corundum_dir is empty
            run(['git', 'clone', 'https://github.com/corundum/corundum.git', corundum_dir])
            run(['git', '-C', corundum_dir, 'reset', '--hard', '45b7e35'])
    else:
        run(['git', 'clone', 'https://github.com/corundum/corundum.git', corundum_dir])
        run(['git', '-C', corundum_dir, 'reset', '--hard', '45b7e35'])


# Copy modules that have individual testbenches into Corundum directory.
def copy_tb_modules():
    for module in tb_modules:
        dirname, basename = get_path_and_name(module)

        run(['cp', shakeflow_dir / 'scripts' / 'rtl' / f'{basename}.v', corundum_dir / dirname])
        run(['cp', shakeflow_dir / 'scripts' / 'tb' / basename / 'Makefile', corundum_dir / 'fpga/common/tb' / basename])
        run(['cp', shakeflow_dir / 'scripts' / 'tb' / basename / f'test_{basename}.py', corundum_dir / 'fpga/common/tb' / basename])

        run(['cp', shakeflow_dir / 'build' / f'{basename}_inner.v', corundum_dir / dirname])


# Copy ShakeFlow modules into Corundum directory.
def copy_test_fpga_core_modules():
    for (val_module_name, cor_module_path) in fpga_tb_module_pairs:
        cor_module_dir, cor_module_name = get_path_and_name(cor_module_path)
        run(['cp', shakeflow_dir / 'scripts' / cor_module_dir.relative_to('fpga/common') / f'{val_module_name}.v',
            corundum_dir / cor_module_dir / f'{cor_module_name}.v'])
        run(['cp', shakeflow_dir / 'build' / f'{val_module_name}_inner.v',
            corundum_dir / cor_module_dir / f'{cor_module_name}_inner.v'])
        if val_module_name != cor_module_name:
            run(['sed', '-i', 
                f's/{val_module_name}/{cor_module_name}/g',
                corundum_dir / cor_module_dir / f'{cor_module_name}_inner.v'])

    for (val_module_name, cor_module_path) in fpga_tb_module_pairs_without_inner:
        cor_module_dir, cor_module_name = get_path_and_name(cor_module_path)
        run(['cp', shakeflow_dir / 'scripts' / cor_module_dir.relative_to('fpga/common') / f'{val_module_name}.v',
            corundum_dir / cor_module_dir / f'{cor_module_name}.v'])

    run(['cp', shakeflow_dir / 'scripts/tb/fpga_core/Makefile', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/tb/fpga_core'])
    run(['cp', shakeflow_dir / 'scripts/tb/fpga_core/test_fpga_core.py', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/tb/fpga_core'])


# Copy bitstream generation modules into Corundum directory.
def copy_bitstream_gen_modules():
    for (val_module_name, cor_module_path) in bitstream_gen_module_pairs:
        cor_module_dir, cor_module_name = get_path_and_name(cor_module_path)
        run(['cp', shakeflow_dir / 'build' / f'{val_module_name}_inner.v',
            corundum_dir / cor_module_dir / f'{cor_module_name}_inner.v'])
        if val_module_name != cor_module_name:
            run(['sed', '-i', 
                f's/{val_module_name}/{cor_module_name}/g',
                corundum_dir / cor_module_dir / f'{cor_module_name}_inner.v'])

    run(['cp', shakeflow_dir / 'scripts/fpga/Makefile', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga'])
    run(['cp', shakeflow_dir / 'scripts/common/vivado.mk', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/common'])


def copy_modules():
    copy_tb_modules()
    copy_test_fpga_core_modules()
    copy_bitstream_gen_modules()


# Parse utilization report for given regex, and return the first number encountered.
def parse_util(report, regex):
    x = list(filter(lambda s: re.search(regex, s), report))[0]
    return re.findall(r"[-+]?\d*\.\d+|\d+", x)[0]


# Return `code` result of `cloc` cmd.
def cloc(path):
    cloc_output = subprocess.run(['cloc', path], check=True, capture_output=True).stdout.decode('utf-8')
    return re.findall(r"[-+]?\d*\.\d+|\d+", cloc_output.splitlines()[-2])[-1]

# reset and set up IP.
def reset_and_setup_machine(mtu):
    if args.mode == 'bench_nic':
        return
    while True:
        subprocess.run(['ssh', '-q', machine_name, 'sudo rmmod mqnic'])
        print('Hot reset', flush=True)
        subprocess.run(['ssh', '-q', machine_name, f'sudo ./corundum/utils/mqnic-fw -d /sys/bus/pci/devices/{alveo_port}/resource0 -b'],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )

        # `make -C corundum/modules/mqnic` doesn't work
        if subprocess.run(['ssh', '-q', machine_name, '''
                cd corundum/modules/mqnic
                make
                cd
                sudo insmod ./corundum/modules/mqnic/mqnic.ko
            ''',],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        ).returncode == 0:
            sleep(3)
        retcode = subprocess.run(['ssh', '-q', machine_name, f'''
                sudo ip addr add {ip_to}/24 dev eth0
                sudo ip link set eth0 up
                sudo ip link set eth0 mtu {mtu}
            '''],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        ).returncode
        if retcode == 0:
            break
        else:
            retcode = subprocess.run(['ssh', '-q', machine_name, f'''
                    sudo ip addr add {ip_to}/24 dev {fpga_alt_pcie}
                    sudo ip link set {fpga_alt_pcie} up
                    sudo ip link set {fpga_alt_pcie} mtu {mtu}
                '''],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            ).returncode
            if retcode == 0:
                break


def bench_connection(num_conn, mtu, half_duplex, txrx):
    if txrx == 'tx':
        machine_snd, machine_rcv = machine_to, machine_from
        ip_snd, ip_rcv = ip_to, ip_from
    else:
        machine_snd, machine_rcv = machine_from, machine_to
        ip_snd, ip_rcv = ip_from, ip_to
    if half_duplex:
        mode_text = 'half-duplex'
        bidir_flag = ''
    else:
        mode_text = 'full_duplex'
        bidir_flag = '-d'

    print(f'{datetime.now()} Testing {mode_text} mtu {mtu} {txrx} with iperf -P {num_conn}:', flush=True)
    retries = 0

    while True:
        sleep(wait_time)
        # reset_and_setup_machine(mtu)
        ps_output = subprocess.run(['ssh', '-q', machine_snd, 'ps aux'], capture_output=True, check=True).stdout.decode('utf-8')
        x = list(filter(lambda s: re.search('iperf', s), ps_output.splitlines()))
        if len(x) > 0:
            pid = re.findall(r"[-+]?\d*\.\d+|\d+", x[0])[0]
            subprocess.run(['ssh', '-q', machine_snd, f'kill -9 {pid}'])
        ps_output = subprocess.run(['ssh', '-q', machine_rcv, 'ps aux'], capture_output=True, check=True).stdout.decode('utf-8')
        x = list(filter(lambda s: re.search('iperf', s), ps_output.splitlines()))
        if len(x) > 0:
            pid = re.findall(r"[-+]?\d*\.\d+|\d+", x[0])[0]
            subprocess.run(['ssh', '-q', machine_rcv, f'kill -9 {pid}'])

        rcv_cmd = ['ssh', '-q', machine_rcv, f'iperf -s -P {num_conn} -c {ip_snd}']
        print(f'rcv: {rcv_cmd}', flush=True)
        subprocess.Popen(['ssh', '-q', machine_rcv, f'iperf -s -P {num_conn} -c {ip_snd}'], 
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
        sleep(wait_time)

        snd_cmd = ['ssh', '-q', machine_snd,
                f'iperf -c {ip_rcv} -P {num_conn} {bidir_flag} -t {bench_time}']
        print(f'snd: {snd_cmd}', flush=True)
        iperf = subprocess.run(snd_cmd, capture_output=True)
        iperf_output = iperf.stdout.decode('utf-8')

        if num_conn == 1:    
            x = list(filter(lambda s: re.search(f'{ip_rcv} port 5001', s), iperf_output.splitlines()))
            # x == ['[  3] local 10.106.41.1 port 34320 connected with 10.106.41.2 port 5001']
            if len(x) == 0:
                continue
            tx_num = re.search(r'\[(.*?)\]', x[0]).group()
            # tx_num == '[  3]'
            x = list(filter(lambda s: re.search('Gbits/sec', s), iperf_output.splitlines()))
            # x == ['[  3]  0.0-10.0 sec  27.6 GBytes  23.7 Gbits/sec', ..]
            x = list(filter(lambda s: re.search(r'\{0}'.format(tx_num), s), x))
            # x == ['[  3]  0.0-10.0 sec  27.6 GBytes  23.7 Gbits/sec']
            if len(x) == 0:
                continue
            gbps = re.findall(r"[-+]?\d*\.\d+|\d+", x[0])[-1]
            # gbps == '23.7'
        else:
            x = list(filter(lambda s: re.search(r'\[SUM\]', s), iperf_output.splitlines()))
            if len(x) == 0:
                continue
            gbps = re.findall(r"[-+]?\d*\.\d+|\d+", x[0])[-1]
        # retry = (num_conn == 1 and float(gbps) < 10.0) or (num_conn == 2 and float(gbps) < 20.0) or (num_conn >= 3 and float(gbps) < 30.0)
        # if retry:
        #     retries += 1
        #     if retries < 3:
        #         continue

        # if subprocess.run(['ssh', '-q', machine_from, f'ping -c 1 {ip_to}'],
        #     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        # ).returncode != 0:
        #     print('ping failed; restarting', flush=True)
        #     reset_and_setup_machine(mtu)
        #     continue
        # if subprocess.run(['ssh', '-q', machine_to, f'ping -c 1 {ip_from}'],
        #     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        # ).returncode != 0:
        #     print('ping failed; restarting', flush=True)
        #     reset_and_setup_machine(mtu)
        #     continue
        f.write(f'{gbps},')
        f.flush()
        os.fsync(f)
        print(f'{gbps} Gbps', flush=True)
        sleep(sleep_time)
        break


# ------------------------ FUNCTIONS END -----------------------

# ---------------------- MAIN STARTS HERE ----------------------

parser = argparse.ArgumentParser(description=help, formatter_class=argparse.RawDescriptionHelpFormatter)
parser.add_argument('mode', choices=['test_cocotb', 'program', 'bench', 'util', 
    'program_per_module', 'port_info', 'bench_nic', 'bench_one', 'setup', 'fio',
    'nginx_wrk', 'nginx_scale', 'setup_nic', 'setup_nginx'])
parser.add_argument('--token', help='CI token')
parser.add_argument('--tb', help='name of tb')
parser.add_argument('--machine', help='name of machine (e.g. `f01`)',
    choices = ['f01', 'f02', 'f03', 'f04', 'f05', 'f06', 'f07', 'j11', 'j04'])
parser.add_argument('--corundum_path', help='path of Corundum directory')
parser.add_argument('--bit', help='path of bitstream file')
parser.add_argument('--tb_exclude', help='excluded tb modules')
parser.add_argument('--name', help='name of generated bench file')
parser.add_argument('--comment', help='comment to be appended to bench file')
parser.add_argument('--server_machine', help='machine that will act as server',
    choices = ['f01', 'f02', 'f03', 'f04', 'f05', 'f06', 'f07', 'j11', 'j12', 'j02'])
parser.add_argument('--mtu')
parser.add_argument('--duplex')
parser.add_argument('--p')
parser.add_argument('--tb_include', help='included tb modules')
args = parser.parse_args()

mode = args.mode
machine_name = args.machine
if mode == 'test_cocotb':
    ci_job_token = args.token
elif mode in ['program', 'bench', 'bench_nic', 'bench_one', 'setup', 'fio', 'nginx_wrk', 
    'nginx_scale', 'setup_nic']:
    if machine_name != None:
        if machine_name == 'f01' or machine_name == 'f02':
            alveo_port = '0000:2b:00.0'
        elif machine_name in ['f03', 'f04', 'f05', 'f06', 'f07']:
            alveo_port = '0000:2d:00.0'
            fpga_alt_pcie = 'enp45s0'
        elif machine_name in ['j11']:
            alveo_port = '0000:41:00.0'
            fpga_alt_pcie = 'enp65s0'
        elif machine_name in ['j04']:
            alveo_port = '0000:25:00.0'
            fpga_alt_pcie = 'enp37s0'
        
        if machine_name in f00_pcie_map:
            f00_pcie = f00_pcie_map[machine_name]
        else:
            f00_pcie = None
        machine_number = int(machine_name.strip(string.ascii_letters))
        if machine_name[0] == 'j':
            machine_number += 10
        ip_from = f'10.{100+machine_number}.41.1'
        ip_to   = f'10.{100+machine_number}.41.2'

if mode in ['test_cocotb', 'program', 'program_per_module', 'port_info']:
    if not (shakeflow_dir / 'scripts/.temp').exists():
        (shakeflow_dir / 'scripts/.temp').mkdir()
    if not (shakeflow_dir / 'build').exists():
        (shakeflow_dir / 'build').mkdir()
    # Remove all files in `.temp`.
    for f in (shakeflow_dir / 'scripts/.temp').iterdir():
        f.unlink()
    # Copy all files from `build` to `.temp`.
    for f in (shakeflow_dir / 'build').iterdir():
        run(['cp', f, shakeflow_dir / 'scripts/.temp'])

    run(['cargo', 'run', '-p', 'shakeflow-corundum'])

if mode in ['bench', 'bench_one', 'setup', 'fio', 'nginx_wrk', 'nginx_scale']:
    if args.server_machine == None:
        machine_from = 'f00'
        pcie = f00_pcie
    else:
        machine_from = args.server_machine
        if machine_from in ['f03', 'f04', 'f05', 'f06', 'f07']:
            if machine_from == 'f05':
                pcie = 'enp45s0f0np0'
            else:
                pcie = 'enp45s0f0'
        elif machine_from == 'j12':
            pcie = 'enp193s0f0'
        elif machine_from == 'j02':
            pcie = 'ens5f0'
        else:
            pcie = 'enp65s0'
        assert(machine_from in ['f03', 'f04', 'f05', 'f06', 'f07', 
                            'j11', 'j12', 'j02'])
    machine_to = machine_name
elif mode in ['bench_nic', 'setup_nic']:
    machine_from = args.server_machine
    # pcie = 'enp45s0f0'
    assert(machine_from in ['f03', 'f04', 'f05', 'f06', 'f07'])
    machine_to = machine_name

    if machine_from == 'f05':
        pcie_from = 'enp45s0f0np0'
    else:
        pcie_from = 'enp45s0f0'
    if machine_to == 'f05':
        pcie_to = 'enp45s0f0np0'
    else:
        pcie_to = 'enp45s0f0'

if mode == 'test_cocotb':
    setup_corundum(args.token) # args.token is `str` or `None`

    if args.tb != None:
        if args.tb == 'fpga_core':
            copy_modules()
            run(['pytest', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])
        else:
            copy_tb_modules()
            tb = list(filter(lambda x: get_path_and_name(x)[1] == args.tb, tb_modules))[0]
            dirname, basename = get_path_and_name(tb)
            run(['pytest', corundum_dir / dirname . parent / 'tb' / basename])
    else:
        # Run module testbenches.
        copy_tb_modules()
        for module in tb_modules:
            dirname, basename = get_path_and_name(module)
            run(['pytest', corundum_dir / dirname . parent / 'tb' / basename])
        # Run test_fpga_core testbench.
        copy_test_fpga_core_modules()
        run(['pytest', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])

elif mode == 'program':
    # If there is diff between current and previous version of Verilog modules,
    # regenerate bitstream. Else, use existing bitstream.
    if args.bit == None:
        if args.tb_exclude != None:
            setup_corundum(None)
            tb_exclude = [m.strip() for m in args.tb_exclude.split(',')]
            copied_modules = list(filter(lambda m: m not in tb_exclude, all_modules))
            # copy `module.v` from `scripts/rtl`.
            for module in copied_modules:
                x = list(filter(lambda p: p[1].split('/')[-1] == module, fpga_tb_module_pairs))
                if len(x) == 0:
                    source_module = module
                elif len(x) == 1:
                    source_module = x[0][0] # e.g. 'rx_checksum_512'
                else:
                    assert(False)
                if (shakeflow_dir / f'scripts/rtl/{source_module}.v').is_file():
                    run(['cp', shakeflow_dir / f'scripts/rtl/{source_module}.v', 
                        corundum_dir / f'fpga/common/rtl/{module}.v'])
                if module in module_replacements:
                    for r in module_replacements[module]:
                        run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v', 
                            corundum_dir / f'fpga/common/rtl/{r}.v'])

            # set up all other `module.v`s with original modules from Corundum.
            for r in module_replacements:
                if r not in copied_modules:
                    for repl in module_replacements[r]:
                        run(['cp', corundum_dir / f'fpga/common/rtl/{r}.v',
                            corundum_dir / f'fpga/common/rtl/{repl}.v'])
                        run(['sed', '-i', 
                            f's/{r}/{repl}/g',
                            corundum_dir / f'fpga/common/rtl/{repl}.v'])
            for module in copied_modules:
                if module in module_replacements:
                    for r in module_replacements[module]:
                        run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v',
                            corundum_dir / f'fpga/common/rtl'])
            for (name, path) in fpga_tb_module_pairs_without_inner:
                run(['cp', shakeflow_dir / f'scripts/rtl/{name}.v', corundum_dir / f'{path}.v'])
        
            # Copy all `*_inner.v`s.
            for f in (shakeflow_dir/ 'build').iterdir():
                run(['cp', f, corundum_dir / 'fpga/common/rtl'])
            for module in copied_modules:
                x = list(filter(lambda p: p[1].split('/')[-1] == module, fpga_tb_module_pairs))
                if len(x) == 0:
                    source_module = module
                elif len(x) == 1:
                    source_module, p = x[0] # e.g. 'rx_checksum_512'
                    run(['cp', shakeflow_dir / f'build/{source_module}_inner.v', corundum_dir / f'{p}_inner.v'])
                    run(['sed', '-i', 
                        f's/{source_module}_inner/{module}_inner/g',
                        corundum_dir / f'{p}_inner.v'])
                else:
                    assert(False)

            # Set up bitstream-gen specific `*_inner.v`s.
            for module in copied_modules:
                if module in module_replacements:
                    for r in module_replacements[module]:
                        bit_modules = list(filter(lambda x: x[1].split('/')[-1] == r, bitstream_gen_module_pairs))
                        if len(bit_modules) != 0:
                            assert(len(bit_modules) == 1)
                            run(['cp', shakeflow_dir / f'build/{bit_modules[0][0]}_inner.v',
                                corundum_dir / f'fpga/common/rtl/{r}_inner.v'])
                            run(['sed', '-i', 
                                f's/{bit_modules[0][0]}_inner/{r}_inner/g',
                                corundum_dir / f'fpga/common/rtl/{r}_inner.v'])
            run(['cp', shakeflow_dir / 'scripts/fpga/Makefile', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga'])
            run(['cp', shakeflow_dir / 'scripts/common/vivado.mk', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/common'])
            run(['make', '-C', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])
        elif args.tb_include != None:
            setup_corundum(None)
            tb_include = [m.strip() for m in args.tb_include.split(',')]
            copied_modules = list(filter(lambda m: m in tb_include, all_modules))
            # copy `module.v` from `scripts/rtl`.
            for module in copied_modules:
                x = list(filter(lambda p: p[1].split('/')[-1] == module, fpga_tb_module_pairs))
                if len(x) == 0:
                    source_module = module
                elif len(x) == 1:
                    source_module = x[0][0] # e.g. 'rx_checksum_512'
                else:
                    assert(False)
                if (shakeflow_dir / f'scripts/rtl/{source_module}.v').is_file():
                    run(['cp', shakeflow_dir / f'scripts/rtl/{source_module}.v', 
                        corundum_dir / f'fpga/common/rtl/{module}.v'])
                if module in module_replacements:
                    for r in module_replacements[module]:
                        run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v', 
                            corundum_dir / f'fpga/common/rtl/{r}.v'])

            # set up all other `module.v`s with original modules from Corundum.
            for r in module_replacements:
                if r not in copied_modules:
                    for repl in module_replacements[r]:
                        run(['cp', corundum_dir / f'fpga/common/rtl/{r}.v',
                            corundum_dir / f'fpga/common/rtl/{repl}.v'])
                        run(['sed', '-i', 
                            f's/{r}/{repl}/g',
                            corundum_dir / f'fpga/common/rtl/{repl}.v'])
            for module in copied_modules:
                if module in module_replacements:
                    for r in module_replacements[module]:
                        run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v',
                            corundum_dir / f'fpga/common/rtl'])
            for (name, path) in fpga_tb_module_pairs_without_inner:
                run(['cp', shakeflow_dir / f'scripts/rtl/{name}.v', corundum_dir / f'{path}.v'])
        
            # Copy all `*_inner.v`s.
            for f in (shakeflow_dir/ 'build').iterdir():
                run(['cp', f, corundum_dir / 'fpga/common/rtl'])
            for module in copied_modules:
                x = list(filter(lambda p: p[1].split('/')[-1] == module, fpga_tb_module_pairs))
                if len(x) == 0:
                    source_module = module
                elif len(x) == 1:
                    source_module, p = x[0] # e.g. 'rx_checksum_512'
                    run(['cp', shakeflow_dir / f'build/{source_module}_inner.v', corundum_dir / f'{p}_inner.v'])
                    run(['sed', '-i', 
                        f's/{source_module}_inner/{module}_inner/g',
                        corundum_dir / f'{p}_inner.v'])
                else:
                    assert(False)

            # Set up bitstream-gen specific `*_inner.v`s.
            for module in copied_modules:
                if module in module_replacements:
                    for r in module_replacements[module]:
                        bit_modules = list(filter(lambda x: x[1].split('/')[-1] == r, bitstream_gen_module_pairs))
                        if len(bit_modules) != 0:
                            assert(len(bit_modules) == 1)
                            run(['cp', shakeflow_dir / f'build/{bit_modules[0][0]}_inner.v',
                                corundum_dir / f'fpga/common/rtl/{r}_inner.v'])
                            run(['sed', '-i', 
                                f's/{bit_modules[0][0]}_inner/{r}_inner/g',
                                corundum_dir / f'fpga/common/rtl/{r}_inner.v'])
            run(['cp', shakeflow_dir / 'scripts/fpga/Makefile', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga'])
            run(['cp', shakeflow_dir / 'scripts/common/vivado.mk', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/common'])
            run(['make', '-C', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])
        else:
            if subprocess.run(['diff', shakeflow_dir / 'build', shakeflow_dir / 'scripts/.temp']).returncode != 0 \
                or not (corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga/fpga.bit').exists():
                if not (corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga/fpga.bit').exists():
                    print('Bitstream not found, generating bitstream')
                setup_corundum(None)
                copy_modules()

                run(['make', '-C', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])

    if args.machine != None:
        # Upload bitstream to designated machine.
        if args.bit == None:
            bitstream = corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga/fpga.bit'
        else:
            bitstream = pathlib.Path(args.bit)
        run(['scp', bitstream, machine_name+':~/fpga.bit'])
        run(['ssh', machine_name, '''
            if [ ! -d ~/corundum ]; then
                git clone https://github.com/corundum/corundum.git
                git reset --hard 45b7e35
            fi
            git -C ~/corundum fetch origin
            git -C ~/corundum reset --hard
        '''])

        # Program the FPGA with given bitstream.
        # TODO: This script currently assumes FPGA program will succeed on first try.
        if subprocess.run(['ssh', machine_name, 
            f'sudo ./corundum/utils/mqnic-fw -d /sys/bus/pci/devices/{alveo_port}/resource0 -w ~/fpga.bit']
        ).returncode != 0:
            print("TODO: This script currently assumes FPGA program will succeed on first try.")
            print("Disclaimer: Rerunning this script will not generate bitstream all over again.")
            exit(1)
        subprocess.run(['ssh', '-q', machine_name, 'sudo rmmod mqnic'])

elif mode == 'bench':
    run(['ssh', '-q', machine_from, f'''
        sudo ip addr add {ip_from}/24 dev {pcie}
        sudo ip link set {pcie} up
        sudo ip link set {pcie} mtu 9000
    '''])

    reset_and_setup_machine(9000)

    # Test rx/tx ping.
    run(['ssh', '-q', machine_from, f'ping -c 1 {ip_to}'])
    run(['ssh', '-q', machine_to, f'ping -c 1 {ip_from}'])

    iters = 0
    now = datetime.now()
    if args.name == None:
        csv_name = f'data_{machine_name}_{now.strftime("%m%d_%H%M")}'
    else:
        csv_name = f'data_{args.name}_{machine_name}_{now.strftime("%m%d_%H%M")}'
    while True:
        csv_path = shakeflow_dir / f'scripts/{csv_name}_{iters}.csv'
        with open(csv_path, 'w') as f:
            for mtu in [1500, 9000]:
                f.write(f'mtu {mtu},\n')
                f.write('parallel connections, tx half-duplex, rx half-duplex, tx full-duplex, rx full-duplex,\n')
                run(['ssh', '-q', machine_from, f'sudo ip link set {pcie} mtu {mtu}'])
                if subprocess.run(['ssh', '-q', machine_to, f'sudo ip link set eth0 mtu {mtu}']).returncode != 0:
                    run(['ssh', '-q', machine_to, f'sudo ip link set {fpga_alt_pcie} mtu {mtu}'])
                for num_conn in range(1, 8+1):
                    f.write(f'{num_conn},')
                    for half_duplex in [True, False]:
                        bench_connection(num_conn, mtu, half_duplex, 'tx')
                        bench_connection(num_conn, mtu, half_duplex, 'rx')
                    f.write('\n')
                f.write('\n')
        iters += 1
elif mode == 'fio':
    run(['ssh', '-q', machine_from, f'''
        sudo ip addr add {ip_from}/24 dev {pcie}
        sudo ip link set {pcie} up
        sudo ip link set {pcie} mtu 9000
    '''])

    reset_and_setup_machine(9000)

    # Test rx/tx ping.
    run(['ssh', '-q', machine_from, f'ping -c 1 {ip_to}'])
    run(['ssh', '-q', machine_to, f'ping -c 1 {ip_from}'])
    
    iters = 0
    now = datetime.now()
    if args.name == None:
        csv_name = f'fio_{machine_name}_{now.strftime("%m%d_%H%M")}'
    else:
        csv_name = f'fio_{args.name}_{machine_name}_{now.strftime("%m%d_%H%M")}'
    
    subprocess.run(['ssh', '-q', machine_to, 'killall fio'])
    subprocess.run(['ssh', '-q', machine_from, 'killall fio'])
    subprocess.Popen(['ssh', '-q', machine_from, f'fio --server'], 
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    run(['scp', '-q', shakeflow_dir / 'scripts/custom.fio', f'{machine_to}:~'])
    fio_timeout = str(90)
    while True:
        csv_path = shakeflow_dir / f'scripts/{csv_name}_{iters}.csv'
        with open(csv_path, 'w') as f:
            for bs in ['4K', '256K']:
                for depth in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096]:
                    sleep(5)
                    f.write(f'blocksize {bs} depth {depth},\n')
                    print(f'run blocksize {bs} depth {depth}', flush=True)
                    print(['timeout', fio_timeout, 'ssh', '-q', machine_to, f"env BLOCKSIZE={bs} IODEPTH={depth} bash -c 'sudo -E perf stat -a -C 0 -e cycles,instructions,cache-misses sudo -E fio --client=ip:{ip_from} ~/custom.fio'"], flush=True)
                    ran = subprocess.run(['timeout', fio_timeout, 'ssh', '-q', machine_to, f"env BLOCKSIZE={bs} IODEPTH={depth} bash -c 'sudo -E perf stat -a -C 0 -e cycles,instructions,cache-misses sudo -E fio --client=ip:{ip_from} ~/custom.fio'"], capture_output=True)
                    while ran.returncode != 0:
                        print("fail; killall", flush=True)
                        subprocess.run(['ssh', '-q', machine_to, 'killall fio'])
                        subprocess.run(['ssh', '-q', machine_from, 'killall fio'])
                        print("sleep 10", flush=True)
                        sleep(10)
                        subprocess.Popen(['ssh', '-q', machine_from, f'fio --server'], 
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
                        )
                        sleep(5)
                        ran = subprocess.run(['timeout', fio_timeout, 'ssh', '-q', machine_to, f"env BLOCKSIZE={bs} IODEPTH={depth} bash -c 'sudo -E perf stat -a -C 0 -e cycles,instructions,cache-misses sudo -E fio --client=ip:{ip_from} ~/custom.fio'"], capture_output=True)
                    err = ran.stderr.decode('utf-8')
                    x = list(filter(lambda s: re.search('cycles', s), err.splitlines()))[0]
                    f.write(f"{x.split()[0]}\n")
                    output = ran.stdout.decode('utf-8')
                    x = list(filter(lambda s: re.search('bw', s), output.splitlines()))[0]
                    f.write(f"{x}\n")
                    x = list(filter(lambda s: re.search('iops', s), output.splitlines()))[0]
                    f.write(f"{x}\n")
                    f.flush()
        iters += 1

elif mode == 'nginx_wrk':
    iters = 0
    now = datetime.now()
    if args.name == None:
        csv_name = f'wrk_{machine_name}_{now.strftime("%m%d_%H%M")}'
    else:
        csv_name = f'wrk_{args.name}_{machine_name}_{now.strftime("%m%d_%H%M")}'
    
    subprocess.run(['ssh', '-q', machine_to, 'sudo killall wrk'])
    fio_timeout = str(90)
    while True:
        csv_path = shakeflow_dir / f'scripts/{csv_name}_{iters}.csv'
        with open(csv_path, 'w') as f:
            for cores in [1, 8]:
                for fsize in [4096, 16384, 65536, 262144, 1048576]:
                    subprocess.run(['ssh', '-q', machine_from, 'sudo killall nginx'])
                    if cores == 1:
                        affinity = "1"
                    else:
                        affinity = "1 100 10000 1000000 100000000 10000000000 1000000000000 100000000000000"
                    subprocess.run(['ssh', '-q', 'machine_from', f'echo {fsize//1024} | sudo tee /sys/block/nvme0n1/queue_read_ahead.kb'])
                    print(['ssh', '-q', machine_from, f'TBASE=~/autonomous-asplos21-artifact/TestSuite/ sudo -E nginx -c ~/autonomous-asplos21-artifact/TestSuite/Tests/nginx/nginx.conf -g "worker_processes {cores}; worker_cpu_affinity {affinity};"'], flush=True)
                    subprocess.Popen(['ssh', '-q', machine_from, f'TBASE=~/autonomous-asplos21-artifact/TestSuite/ sudo -E nginx -c ~/autonomous-asplos21-artifact/TestSuite/Tests/nginx/nginx.conf -g "worker_processes {cores}; worker_cpu_affinity {affinity};"'], 
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
                    )
                    sleep(5)
                    f.write(f'cores {cores} fsize {fsize},\n')
                    print(f'run cores {cores} fsize {fsize}', flush=True)
                    print(['ssh', '-q', machine_to, f"maxpaths={2147483648//fsize - 1} mode=https fsize={fsize} dip1={ip_from} ~/wrk/wrk --timeout 100 -t 16 -c 128 -d 70 -s ~/wrk/http.size.lua https://{ip_from}"], flush=True)
                    ran = subprocess.run(['ssh', '-q', machine_to, f"maxpaths={2147483648//fsize - 1} mode=https fsize={fsize} dip1={ip_from} ~/wrk/wrk --timeout 100 -t 16 -c 128 -d 70 -s ~/wrk/http.size.lua https://{ip_from}"], capture_output=True)
                    while ran.returncode != 0:
                        assert False
                        # print("fail; killall", flush=True)
                        # subprocess.run(['ssh', '-q', machine_to, 'killall fio'])
                        # subprocess.run(['ssh', '-q', machine_from, 'killall fio'])
                        # print("sleep 10", flush=True)
                        # sleep(10)
                        # subprocess.Popen(['ssh', '-q', machine_from, f'fio --server'], 
                        #     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
                        # )
                        # sleep(5)
                        # ran = subprocess.run(['timeout', fio_timeout, 'ssh', '-q', machine_to, f"env BLOCKSIZE={bs} IODEPTH={depth} bash -c 'sudo -E perf stat -a -C 0 -e cycles,instructions,cache-misses sudo -E fio --client=ip:{ip_from} ~/custom.fio'"], capture_output=True)
                    f.write(ran.stderr.decode('utf-8'))
                    f.write(ran.stdout.decode('utf-8'))
                    f.flush()
        iters += 1
elif mode == 'nginx_scale':
    iters = 0
    now = datetime.now()
    if args.name == None:
        csv_name = f'scale_wrk_{machine_name}_{now.strftime("%m%d_%H%M")}'
    else:
        csv_name = f'scale_wrk_{args.name}_{machine_name}_{now.strftime("%m%d_%H%M")}'
    
    subprocess.run(['ssh', '-q', machine_to, 'sudo killall wrk'])
    fio_timeout = str(90)
    while True:
        csv_path = shakeflow_dir / f'scripts/{csv_name}_{iters}.csv'
        with open(csv_path, 'w') as f:
            cores = 8
            subprocess.run(['ssh', '-q', machine_from, 'sudo killall nginx'])
            affinity = "1 100 10000 1000000 100000000 10000000000 1000000000000 100000000000000"
            subprocess.Popen(['ssh', '-q', machine_from, f'TBASE=~/autonomous-asplos21-artifact/TestSuite/ sudo -E nginx -c ~/autonomous-asplos21-artifact/TestSuite/Tests/nginx/nginx.conf -g "worker_processes {cores}; worker_cpu_affinity {affinity};"'], 
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            )
            fsize = 262144
            sleep(5)
            for conn in [64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072]:
                f.write(f'conns {conn} cores {cores} fsize {fsize},\n')
                print(f'run conns {conn} cores {cores} fsize {fsize}', flush=True)
                print(['ssh', '-q', machine_to, f"maxpaths={2147483648//fsize - 1} mode=https fsize={fsize} dip1={ip_from} ~/wrk/wrk --timeout 100 -t 16 -c {conn} -d 70 -s ~/wrk/http.size.lua https://{ip_from}"], flush=True)
                ran = subprocess.run(['ssh', '-q', machine_to, f"maxpaths={2147483648//fsize - 1} mode=https fsize={fsize} dip1={ip_from} ~/wrk/wrk --timeout 100 -t 16 -c 128 -d 70 -s ~/wrk/http.size.lua https://{ip_from}"], capture_output=True)
                while ran.returncode != 0:
                    assert False
                    # print("fail; killall", flush=True)
                    # subprocess.run(['ssh', '-q', machine_to, 'killall fio'])
                    # subprocess.run(['ssh', '-q', machine_from, 'killall fio'])
                    # print("sleep 10", flush=True)
                    # sleep(10)
                    # subprocess.Popen(['ssh', '-q', machine_from, f'fio --server'], 
                    #     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
                    # )
                    # sleep(5)
                    # ran = subprocess.run(['timeout', fio_timeout, 'ssh', '-q', machine_to, f"env BLOCKSIZE={bs} IODEPTH={depth} bash -c 'sudo -E perf stat -a -C 0 -e cycles,instructions,cache-misses sudo -E fio --client=ip:{ip_from} ~/custom.fio'"], capture_output=True)
                f.write(ran.stderr.decode('utf-8'))
                f.write(ran.stdout.decode('utf-8'))
                f.flush()
        iters += 1
elif mode == 'bench_nic' or mode == 'setup_nic':
    run(['ssh', '-q', machine_from, f'''
        sudo ip addr add {ip_from}/24 dev {pcie_from}
        sudo ip link set {pcie_from} up
        sudo ip link set {pcie_from} mtu 9000
    '''])

    run(['ssh', '-q', machine_to, f'''
        sudo ip addr add {ip_to}/24 dev {pcie_to}
        sudo ip link set {pcie_to} up
        sudo ip link set {pcie_to} mtu 9000
    '''])

    # Test rx/tx ping.
    run(['ssh', '-q', machine_from, f'ping -c 1 {ip_to}'])
    run(['ssh', '-q', machine_to, f'ping -c 1 {ip_from}'])

    if mode == 'bench_nic':
        iters = 0
        now = datetime.now()
        if args.name == None:
            csv_name = f'data_{machine_name}_{now.strftime("%m%d_%H%M")}'
        else:
            csv_name = f'data_{args.name}_{machine_name}_{now.strftime("%m%d_%H%M")}'
        while True:
            csv_path = shakeflow_dir / f'scripts/{csv_name}_{iters}.csv'
            with open(csv_path, 'w') as f:
                for mtu in [1500]:
                    f.write(f'mtu {mtu},\n')
                    f.write('parallel connections, tx half-duplex, rx half-duplex, tx full-duplex, rx full-duplex,\n')
                    run(['ssh', '-q', machine_from, f'sudo ip link set {pcie_from} mtu {mtu}'])
                    run(['ssh', '-q', machine_to, f'sudo ip link set {pcie_to} mtu {mtu}'])
                    for num_conn in range(1, 8+1):
                        f.write(f'{num_conn},')
                        for half_duplex in [True, False]:
                            bench_connection(num_conn, mtu, half_duplex, 'tx')
                            bench_connection(num_conn, mtu, half_duplex, 'rx')
                        f.write('\n')
                    f.write('\n')
            iters += 1

elif mode == 'bench_one' or mode == 'setup':
    run(['ssh', '-q', machine_from, f'''
        sudo ip addr add {ip_from}/24 dev {pcie}
        sudo ip link set {pcie} up
        sudo ip link set {pcie} mtu 9000
    '''])

    if args.mtu == None:
        mtu = 9000
    else:
        mtu = int(args.mtu)
    half_duplex = not args.duplex == 'full'
    if args.p == None:
        num_conn = 5
    else:
        num_conn = int(args.p)

    subprocess.run(['ssh', '-q', machine_name, '''
        sudo rmmod mqnic
    '''])
    # `make -C corundum/modules/mqnic` doesn't work
    if subprocess.run(['ssh', '-q', machine_name, 'test -d corundum']).returncode != 0:
        run(['ssh', machine_name, '''
            if [ ! -d ~/corundum ]; then
                git clone https://github.com/corundum/corundum.git
                git -C corundum reset --hard 45b7e35
            fi
            git -C ~/corundum fetch origin
            git -C ~/corundum reset --hard
        '''])
    subprocess.run(['ssh', '-q', machine_name, '''
        cd corundum/modules/mqnic
        make
        cd
        sudo insmod ./corundum/modules/mqnic/mqnic.ko
    '''])
    sleep(3)
    retcode = subprocess.run(['ssh', '-q', machine_name, f'''
        sudo ip addr add {ip_to}/24 dev eth0
        sudo ip link set eth0 up
        sudo ip link set eth0 mtu {mtu}
    ''']).returncode
    if retcode != 0:
        retcode = subprocess.run(['ssh', '-q', machine_name, f'''
            sudo ip addr add {ip_to}/24 dev {fpga_alt_pcie}
            sudo ip link set {fpga_alt_pcie} up
            sudo ip link set {fpga_alt_pcie} mtu {mtu}
        ''']).returncode

    if mode == 'bench_one':
        with open('temp.csv', 'w') as f:
            run(['ssh', '-q', machine_from, f'sudo ip link set {pcie} mtu {mtu}'])
            if subprocess.run(['ssh', '-q', machine_to, f'sudo ip link set eth0 mtu {mtu}']).returncode != 0:
                run(['ssh', '-q', machine_to, f'sudo ip link set {fpga_alt_pcie} mtu {mtu}'])
            bench_connection(num_conn, mtu, half_duplex, 'tx')
            bench_connection(num_conn, mtu, half_duplex, 'rx')
    elif mode == 'setup':
        print('Setup complete!')

elif mode == 'util':
    # Assumes `program` was already called and bitstream file & report exists.
    if args.corundum_path == None:
        report_dir = corundum_dir
    else:
        report_dir = pathlib.Path(args.corundum_path)
    report_path = report_dir / 'fpga/mqnic/AU200/fpga_100g/fpga/fpga.runs/impl_1/fpga_utilization_placed.rpt'
    report = open(report_path, 'r').read().splitlines()

    # Parse utilization report.
    luts = parse_util(report, r'CLB LUTs')
    luts_l = parse_util(report, r'LUT as Logic')
    luts_m = parse_util(report, r'LUT as Memory')
    ffs = parse_util(report, r'CLB Registers')
    bram = parse_util(report, r'Block RAM Tile')
    uram = parse_util(report, r'URAM')
     
    # Parse timing summary.
    report_path = report_dir / 'fpga/mqnic/AU200/fpga_100g/fpga/fpga.runs/impl_1/fpga_timing_summary_routed.rpt'
    report = open(report_path, 'r').read().splitlines()
    
    summary_index = int(list(filter(lambda s: re.search(r'Design Timing Summary', s[1]), enumerate(report)))[0][0])
    summary_values = re.findall(r"[-+]?\d*\.\d+|\d+", report[summary_index+6])
    wns, tns = summary_values[0], summary_values[1]
    print(f'LUTs: {luts}')
    print(f'  LUT as Logic: {luts_l}')
    print(f'  LUT as Memory: {luts_m}')
    print(f'FFs: {ffs}')
    print(f'BRAM: {bram}')
    print(f'URAM: {uram}')
    print(f'WNS: {wns}')
    print(f'TNS: {tns}')

elif mode == 'program_per_module':
    if args.tb == None:
        search_modules = all_modules
    else:
        search_modules = args.tb.split(',')

    for module in search_modules:
        setup_corundum(None)
        # Note: We are currently assuming that all modules are in `fpga/common/rtl`!

        # copy `module.v` from `scripts/rtl`.
        x = list(filter(lambda p: p[1].split('/')[-1] == module, fpga_tb_module_pairs))
        if len(x) == 0:
            source_module = module
        elif len(x) == 1:
            source_module = x[0][0] # e.g. 'rx_checksum_512'
        else:
            assert(False)
        if (shakeflow_dir / f'scripts/rtl/{source_module}.v').is_file():
            run(['cp', shakeflow_dir / f'scripts/rtl/{source_module}.v', 
                corundum_dir / f'fpga/common/rtl/{module}.v'])
        if module in module_replacements:
            for r in module_replacements[module]:
                run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v', 
                    corundum_dir / f'fpga/common/rtl/{r}.v'])
        
        # set up all other `module.v`s with original modules from Corundum.
        for r in module_replacements:
            if r != module:
                for repl in module_replacements[r]:
                    run(['cp', corundum_dir / f'fpga/common/rtl/{r}.v',
                        corundum_dir / f'fpga/common/rtl/{repl}.v'])
                    run(['sed', '-i', 
                        f's/{r}/{repl}/g',
                        corundum_dir / f'fpga/common/rtl/{repl}.v'])
        if module in module_replacements:
            for r in module_replacements[module]:
                run(['cp', shakeflow_dir / f'scripts/rtl/{r}.v',
                    corundum_dir / f'fpga/common/rtl'])
        for (name, path) in fpga_tb_module_pairs_without_inner:
            run(['cp', shakeflow_dir / f'scripts/rtl/{name}.v', corundum_dir / f'{path}.v'])

        # Copy all `*_inner.v`s.
        for f in (shakeflow_dir/ 'build').iterdir():
            run(['cp', f, corundum_dir / 'fpga/common/rtl'])
        if source_module != module:
            run(['cp', shakeflow_dir / f'build/{source_module}_inner.v', corundum_dir / f'fpga/common/rtl/{module}_inner.v'])
            run(['sed', '-i', 
                f's/{source_module}_inner/{module}_inner/g',
                corundum_dir / f'fpga/common/rtl/{module}_inner.v'])
                
        # Set up bitstream-gen specific `*_inner.v`s.
        if module in module_replacements:
            for r in module_replacements[module]:
                bit_modules = list(filter(lambda x: x[1].split('/')[-1] == r, bitstream_gen_module_pairs))
                if len(bit_modules) != 0:
                    assert(len(bit_modules) == 1)
                    run(['cp', shakeflow_dir / f'build/{bit_modules[0][0]}_inner.v',
                        corundum_dir / f'fpga/common/rtl/{r}_inner.v'])
                    run(['sed', '-i', 
                        f's/{bit_modules[0][0]}_inner/{r}_inner/g',
                        corundum_dir / f'fpga/common/rtl/{r}_inner.v'])

        run(['cp', shakeflow_dir / 'scripts/fpga/Makefile', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/fpga'])
        run(['cp', shakeflow_dir / 'scripts/common/vivado.mk', corundum_dir / 'fpga/mqnic/AU200/fpga_100g/common'])

        print(f'Currently programming {module} module...', flush=True)
        subprocess.run(['make', '-C', corundum_dir / 'fpga/mqnic/AU200/fpga_100g'])
        print(f'Finished programming {module} module.', flush=True)
        run(['mv', shakeflow_dir / 'corundum', shakeflow_dir / f'corundum-{module}'])

elif mode == 'port_info':
    if not (shakeflow_dir / 'scripts/corundum-original').is_dir():
        run(['git', 'clone', 'https://github.com/corundum/corundum.git', shakeflow_dir / 'scripts/corundum-original'])
        run(['git', '-C', shakeflow_dir / 'scripts/corundum-original', 'reset', '--hard', '45b7e35'])
    # module name, SLOC in Verilog, ShakeFlow, Verilog (generated from ShakeFlow)
    print('\\hline')
    print('Module & $\\textsf{C}_{\\textrm{Original}}$ & $\\textsf{C}_{\\textrm{\\shakeflow}}$ Rust & $\\textsf{C}_{\\textrm{\\shakeflow}}$ Verilog \\\\ [0.5ex]')
    print('\\hline\\hline')
    port_infos = set()
    # Assumes that all modules are in `fpga/common/rtl`!
    for module in all_modules:
        corundum_verilog_loc = cloc(shakeflow_dir / f'scripts/corundum-original/fpga/common/rtl/{module}.v')

        module_names = [module]
        shakeflow_rust_locs = []
        shakeflow_verilog_locs = []
        if module in module_replacements:
            module_names.extend(module_replacements[module])
        for module_variation in module_names:
            p = shakeflow_dir / f'shakeflow-corundum/src/{module_variation}.rs'
            if p.is_file():
                shakeflow_rust_locs.append(cloc(p))
            for m in list(filter(lambda x: x[1].split('/')[-1] == module_variation, fpga_tb_module_pairs)):
                p = shakeflow_dir / f'shakeflow-corundum/src/{m[0]}.rs'
                if p.is_file():
                    shakeflow_rust_locs.append(cloc(p))
            for m in list(filter(lambda x: x[1].split('/')[-1] == module_variation, bitstream_gen_module_pairs)):
                p = shakeflow_dir / f'shakeflow-corundum/src/{m[0]}.rs'
                if p.is_file():
                    shakeflow_rust_locs.append(cloc(p))

            p = shakeflow_dir / f'build/{module_variation}_inner.v'
            if p.is_file():
                shakeflow_verilog_locs.append(cloc(p))
            for m in list(filter(lambda x: x[1].split('/')[-1] == module_variation, fpga_tb_module_pairs)):
                p = shakeflow_dir / f'build/{module_variation}_inner.v'
                if p.is_file():
                    shakeflow_verilog_locs.append(cloc(p))
            for m in list(filter(lambda x: x[1].split('/')[-1] == module_variation, bitstream_gen_module_pairs)):
                p = shakeflow_dir / f'build/{module_variation}_inner.v'
                if p.is_file():
                    shakeflow_verilog_locs.append(cloc(p))

        shakeflow_rust_loc = max(shakeflow_rust_locs)
        shakeflow_verilog_loc = max(shakeflow_verilog_locs)
        
        module_tex = '\\code{' + module.replace('_', '\\_') + '}'
        port_infos.add((module, module_tex, corundum_verilog_loc, shakeflow_rust_loc, shakeflow_verilog_loc))

    port_infos = sorted(port_infos, key=lambda x: x[0])
    for info in port_infos:
        print(' & '.join(info[1:]) + ' \\\\')
        print('\\hline')
elif mode == 'setup_nginx':
    run(['ssh', machine_name, '''
        if [ ! -d ~/autonomous-asplos21-artifact ]; then
            git clone https://github.com/BorisPis/autonomous-asplos21-artifact.git
            git -C ~/autonomous-asplos21-artifact reset --hard c0a0347
            rmdir ~/autonomous-asplos21-artifact/TestSuite
            git -C ~/autonomous-asplos21-artifact clone https://github.com/BorisPis/autonomous-asplos21-TestSuite.git
            mv ~/autonomous-asplos21-artifact/autonomous-asplos21-TestSuite ~/autonomous-asplos21-artifact/TestSuite
        fi
        if [ ! -d ~/autonomous-asplos21-wrk ]; then
            git clone https://github.com/BorisPis/wrk.git
            git -C ~/wrk reset --hard e0f7b9d
            make -C ~/wrk
        fi
    '''])
    run(['scp', shakeflow_dir / 'scripts/nginx/generate_files.sh', f'{machine_name}:~/autonomous-asplos21-artifact/TestSuite/Tests/nginx'])
    run(['scp', shakeflow_dir / 'scripts/nginx/run_test.sh', f'{machine_name}:~/autonomous-asplos21-artifact/TestSuite/Tests/nginx'])
    run(['ssh', machine_name, 'TBASE=~/autonomous-asplos21-artifact/TestSuite ~/autonomous-asplos21-artifact-temp/TestSuite/Tests/nginx/generate_all_files.sh'])
