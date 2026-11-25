package com.nordtorrent.tv

import android.annotation.SuppressLint
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.ProgressBar
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView

class TransferAdapter(
    private val onClick: (TorrentSummary) -> Unit,
) : ListAdapter<TorrentSummary, TransferAdapter.TransferViewHolder>(Diff) {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): TransferViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_transfer, parent, false)
        return TransferViewHolder(view)
    }

    override fun onBindViewHolder(holder: TransferViewHolder, position: Int) {
        holder.bind(getItem(position))
    }

    @SuppressLint("SetTextI18n")
    inner class TransferViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val icon: ImageView = itemView.findViewById(R.id.transfer_icon)
        private val title: TextView = itemView.findViewById(R.id.transfer_title)
        private val meta: TextView = itemView.findViewById(R.id.transfer_meta)
        private val status: TextView = itemView.findViewById(R.id.transfer_status)
        private val labelChip: TextView = itemView.findViewById(R.id.transfer_label)
        private val percent: TextView = itemView.findViewById(R.id.transfer_percent)
        private val speed: TextView = itemView.findViewById(R.id.transfer_speed)
        private val progress: ProgressBar = itemView.findViewById(R.id.transfer_progress)

        init {
            itemView.isFocusable = true
            itemView.isFocusableInTouchMode = true
            (itemView as ViewGroup).descendantFocusability = ViewGroup.FOCUS_BLOCK_DESCENDANTS
            itemView.setOnFocusChangeListener { _, hasFocus ->
                // Do not scale full-width transfer rows: scaling changes the
                // card's horizontal bounds and makes the focused row look
                // wider than every other row (and clips at the list edge).
                itemView.animate()
                    .translationZ(if (hasFocus) 6f else 0f)
                    .setDuration(if (hasFocus) 150 else 110)
                    .start()
            }
        }

        fun bind(torrent: TorrentSummary) {
            title.text = torrent.name.ifBlank { itemView.context.getString(R.string.unknown_torrent) }
            meta.text = "${formatBytes(torrent.progressBytes)} / ${formatBytes(torrent.totalBytes)}"
            status.text = torrentStateLabel(itemView.context, torrent)
            status.setTextColor(torrentStateColor(torrent))
            if (torrent.label.isBlank()) {
                labelChip.visibility = View.GONE
            } else {
                labelChip.visibility = View.VISIBLE
                labelChip.text = torrent.label
            }
            percent.text = "${progressPercent(torrent.progress)}%"
            val speedText = when {
                torrent.error != null -> torrent.error
                torrent.downloadSpeedBps > 1 -> "↓ ${formatSpeed(torrent.downloadSpeedBps)}"
                torrent.uploadSpeedBps > 1 -> "↑ ${formatSpeed(torrent.uploadSpeedBps)}"
                torrent.state == "paused" -> itemView.context.getString(R.string.state_paused)
                else -> itemView.context.getString(R.string.no_traffic)
            }
            speed.text = speedText
            progress.progress = progressPercent(torrent.progress)
            progress.progressTintList =
                android.content.res.ColorStateList.valueOf(torrentStateColor(torrent))
            icon.setImageResource(
                when {
                    torrent.state == "error" -> R.drawable.ic_alert
                    torrent.mediaFiles > 0 -> R.drawable.ic_film
                    else -> R.drawable.ic_music
                },
            )
            icon.imageTintList = android.content.res.ColorStateList.valueOf(torrentStateColor(torrent))
            itemView.contentDescription = itemView.context.getString(
                R.string.transfer_content_desc,
                torrent.name.ifBlank { itemView.context.getString(R.string.unknown_torrent) },
                torrentStateLabel(itemView.context, torrent),
            )
            itemView.setOnClickListener { onClick(torrent) }
        }
    }

    private object Diff : DiffUtil.ItemCallback<TorrentSummary>() {
        override fun areItemsTheSame(oldItem: TorrentSummary, newItem: TorrentSummary): Boolean {
            return oldItem.id == newItem.id
        }

        override fun areContentsTheSame(oldItem: TorrentSummary, newItem: TorrentSummary): Boolean {
            return oldItem == newItem
        }
    }
}
