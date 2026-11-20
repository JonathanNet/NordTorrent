package com.nordtorrent.tv

import android.app.Activity
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Android 15 draws app content edge-to-edge by default.  Keep the phone UI
 * below the status bar and above the navigation bar without affecting TV
 * devices (which normally report zero-sized system bars).
 */
fun applySystemBarInsets(activity: Activity) {
    val root = activity.findViewById<View>(android.R.id.content) ?: return
    ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
        insets
    }
    ViewCompat.requestApplyInsets(root)
}
