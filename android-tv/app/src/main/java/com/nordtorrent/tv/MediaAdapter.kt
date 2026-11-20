package com.nordtorrent.tv

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.ProgressBar
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView

class MediaAdapter(
    private val onClick: (LibraryItem) -> Unit,
) : ListAdapter<LibraryItem, MediaAdapter.MediaViewHolder>(Diff) {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): MediaViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_media, parent, false)
        return MediaViewHolder(view)
    }

    override fun onBindViewHolder(holder: MediaViewHolder, position: Int) {
        holder.bind(getItem(position))
    }

    inner class MediaViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val art: View = itemView.findViewById(R.id.media_art)
        private val initial: TextView = itemView.findViewById(R.id.media_initial)
        private val title: TextView = itemView.findViewById(R.id.media_title)
        private val meta: TextView = itemView.findViewById(R.id.media_meta)
        private val badge: TextView = itemView.findViewById(R.id.media_badge)
        private val kindIcon: ImageView = itemView.findViewById(R.id.media_kind_icon)
        private val play: ImageView = itemView.findViewById(R.id.media_play)
        private val progress: ProgressBar = itemView.findViewById(R.id.media_progress)

        init {
            itemView.isFocusable = true
            itemView.isFocusableInTouchMode = true
            (itemView as ViewGroup).descendantFocusability = ViewGroup.FOCUS_BLOCK_DESCENDANTS
            itemView.setOnFocusChangeListener { _, hasFocus ->
                itemView.animate()
                    .scaleX(if (hasFocus) 1.04f else 1f)
                    .scaleY(if (hasFocus) 1.04f else 1f)
                    .setDuration(if (hasFocus) 150 else 110)
                    .start()
                play.imageTintList = ColorStateList.valueOf(
                    if (hasFocus) Color.parseColor("#10160E") else Color.WHITE,
                )
                play.background?.mutate()?.setTint(
                    if (hasFocus) Color.parseColor("#C6F45C") else Color.parseColor("#66080B0E"),
                )
            }
        }

        fun bind(item: LibraryItem) {
            title.text = item.title.ifBlank { item.fileName }
            meta.text = itemView.context.getString(
                R.string.media_meta,
                formatBytes(item.size),
                progressPercent(item.progress),
            )
            initial.text = item.title.trim().take(1).uppercase()
            badge.text = itemView.context.getString(
                if (item.kind == "audio") R.string.media_badge_audio else R.string.media_badge_video,
            )
            kindIcon.setImageResource(if (item.kind == "audio") R.drawable.ic_music else R.drawable.ic_film)
            progress.progress = progressPercent(item.progress)

            val hue = ((item.title.hashCode() and 0x7fffffff) % 360).toFloat()
            val top = Color.HSVToColor(floatArrayOf(hue, 0.46f, 0.36f))
            val bottom = Color.HSVToColor(floatArrayOf((hue + 42f) % 360f, 0.55f, 0.13f))
            art.background = GradientDrawable(
                GradientDrawable.Orientation.TL_BR,
                intArrayOf(top, bottom),
            )

            itemView.contentDescription = itemView.context.getString(
                R.string.media_play_item,
                item.title.ifBlank { item.fileName },
            )
            itemView.setOnClickListener { onClick(item) }
        }
    }

    private object Diff : DiffUtil.ItemCallback<LibraryItem>() {
        override fun areItemsTheSame(oldItem: LibraryItem, newItem: LibraryItem): Boolean {
            return oldItem.torrentId == newItem.torrentId && oldItem.fileId == newItem.fileId
        }

        override fun areContentsTheSame(oldItem: LibraryItem, newItem: LibraryItem): Boolean {
            return oldItem == newItem
        }
    }
}
