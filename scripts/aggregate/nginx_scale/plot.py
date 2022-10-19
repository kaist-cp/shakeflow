# Takes the csv files outputed by `aggregate.py`,
# and creates plot svg and png files.

import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import pandas as pd
import pathlib
import os

COR = "$\\mathbf{C_{Orig}}$"
SF = "$\\mathbf{C_{SF}}$"
MEL = "$\\mathbf{M}$"

orange = '#ff7f0e'
green = '#2ca02c'

plots = [COR, SF, MEL]
point_color = {
    COR : orange,
    SF : 'navy',
    MEL : green,
}

filename = f'scale'
current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
dt = pd.read_csv(f'{current_dir}/{filename}.csv', sep='\t')
dt['type'] = pd.Categorical(dt['type'], categories=plots, ordered=True)

sns.set_theme(style="ticks", font_scale=1.5)

fig = plt.figure()
ax = plt.axes()

# bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color)
bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color, ci='sd')

# Define some hatches
hatches = ["",  "", "xx",]

# Loop over the bars
for bars, hatch in zip(ax.containers, hatches):
    # Set a different hatch for each group of bars
    for b in bars:
        b.set_hatch(hatch)
# create the legend again to show the new hatching
ax.legend()
# plt.show()
bar.set_xlabel('file size [KiB]')
bar.set_ylabel('Throughput [Gbps]')

# remove every other xtick
xlabels = ax.get_xticklabels()
for i, l in enumerate(xlabels):
    if i%2 == 1:
        xlabels[i] = None
ax.set_xticklabels(xlabels)
xs = list(filter(lambda x: x%2==0, ax.get_xticks()))
ax.set_xticks(xs)

plt.tight_layout()
plt.savefig(f'{current_dir}/{filename}.png')
plt.savefig(f'{current_dir}/{filename}.svg')


# p = ggplot(dt, aes(x='size', y='throughput', color='type', fill='type')) + \
#     labs(y='Throughput', x="file size [KiB]", size=element_text(size=20)) + \
#     geom_errorbar(aes(x='size', ymin='throughput-std', ymax='throughput+std')) + \
#     geom_col(stat='identity', position='dodge') + \
#     theme_classic() + \
#     scale_color_manual(point_color) + \
#     scale_fill_manual(point_color) + \
#     scale_x_log10() + \
#     theme(
#         axis_text=element_text(size=24, color='black'),
#         axis_title_x=element_text(size=24),
#         axis_title_y=element_text(size=24),
#         axis_ticks_length = 8,
#         legend_position=(0.65, 0.2), 
#         legend_title=element_blank(),
#         legend_text=element_text(size=28),
#         legend_entry_spacing=20,
#         legend_background=element_rect(color='#d5d5d5', size=2.3, 
#             boxstyle="round,pad=0,rounding_size=0.5"),
#         panel_border=element_rect(color='black', fill=None, size=2)
#     )
#     # coord_cartesian(xlim=(1,8), ylim=(0,100)) + \
#     # scale_y_continuous(breaks=range(0, 101, 20), expand=(0,0)) + \
#     # guides(color=guide_legend(nrow=2)) + \

# p.save(filename+'.svg', width=10, height=10, units="in")
# p.save(filename+'.png', width=10, height=10, units="in")
