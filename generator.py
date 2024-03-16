import random

seq = open("from_sequencer.txt", "+w")
da = open("from_da.txt", "+w")

# generate 200 seq blocks
# the format will be key1 value1, key2 value2, ....
# key is between 0 and 255, value is between 0 and uint64.max
# every five L2 block changes will be merged and written to da in the same format

num_reorgs = 0
num_seq_lied = 0

da_blocks = 0

da_changes = {}
for l2_block in range(0, 200):
    num_change = random.randint(1, 100)

    block_changes = []
    for i in range(0, num_change):
        key = random.randint(0, 255)
        value = random.randint(0, 18446744073709551615) # rust u64 max
        block_changes.append(f"{key} {value}")

        # %0.02 chance sequencer lied
        if random.random() <= 0.0002 and num_seq_lied < 3:
            num_seq_lied += 1
            print("sequencer lied to nodes and wrote the original to da")
            da_changes[key] = value // random.randint(1, 5)
        else:
            da_changes[key] = value

    block_change = ", ".join(block_changes)

    seq.write(block_change + "\n")

    if l2_block % 5 == 4:
        print("writing to da")
        to_da = []
        for key, value in da_changes.items():
            to_da.append(f"{key} {value}")

        to_da_change = ", ".join(to_da)
        da.write(to_da_change + "\n")

        da_changes.clear()

        # %1 chance reorg happens
        if random.random() <= 0.1 and num_reorgs == 0 and da_blocks > 10:
            num_reorgs += 1
            print("reorg happened")
            reorg_length = random.randint(1, 3)

            da.write(f"REORG {reorg_length}\n")

            reorg_changes = {}
            # different chanegs appears for the reorged blocks
            for i in range(0, reorg_length):
                num_change = random.randint(200, 300)
                for i in range(0, num_change):
                    key = random.randint(0, 255)
                    value = random.randint(0, 18446744073709551615) # rust u64 max

                    reorg_changes[key] = value
                
                to_da = []
                for key, value in reorg_changes.items():
                    to_da.append(f"{key} {value}")
                
                to_da_change = ", ".join(to_da)
                print("after reorg a new block to da:", to_da_change)
                da.write(to_da_change + "\n")
                reorg_changes.clear()

        da_blocks += 1