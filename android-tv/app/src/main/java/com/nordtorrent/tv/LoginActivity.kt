package com.nordtorrent.tv

import android.content.Intent
import android.os.Bundle
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.isVisible
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class LoginActivity : AppCompatActivity() {
    private lateinit var store: SessionStore
    private lateinit var serverInput: EditText
    private lateinit var usernameInput: EditText
    private lateinit var passwordInput: EditText
    private lateinit var loginButton: Button
    private lateinit var loginError: TextView
    private lateinit var setupHint: TextView
    private lateinit var progress: ProgressBar
    private var busy = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_login)
        applySystemBarInsets(this)
        store = SessionStore(this)

        serverInput = findViewById(R.id.server_input)
        usernameInput = findViewById(R.id.username_input)
        passwordInput = findViewById(R.id.password_input)
        loginButton = findViewById(R.id.login_button)
        loginError = findViewById(R.id.login_error)
        setupHint = findViewById(R.id.setup_hint)
        progress = findViewById(R.id.login_progress)

        serverInput.setText(store.baseUrl().ifBlank { "http://192.168.1.10:8080" })
        usernameInput.setText(store.username())
        loginButton.setOnClickListener { attemptLogin() }
        passwordInput.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_DONE) {
                attemptLogin()
                true
            } else {
                false
            }
        }

        when {
            store.hasToken() -> validateStoredSession()
            else -> {
                focusFirstField()
                checkServerStatus()
            }
        }
    }

    private fun focusFirstField() {
        if (usernameInput.text.isNullOrBlank()) {
            usernameInput.requestFocus()
        } else {
            passwordInput.requestFocus()
        }
    }

    private fun checkServerStatus() {
        val base = runCatching { ApiClient.normalizeBaseUrl(serverInput.text.toString()) }.getOrNull() ?: return
        lifecycleScope.launch {
            runCatching { ApiClient(base, context = this@LoginActivity).status() }
                .onSuccess { status ->
                    setupHint.isVisible = status.setupRequired
                    if (status.setupRequired) {
                        setupHint.text = getString(R.string.setup_hint)
                    }
                }
        }
    }

    private fun validateStoredSession() {
        val base = store.baseUrl()
        val token = store.token()
        if (base.isBlank() || token.isNullOrBlank()) {
            store.clear()
            focusFirstField()
            return
        }
        setBusy(true, getString(R.string.login_checking))
        lifecycleScope.launch {
            try {
                val user = ApiClient(base, token, context = this@LoginActivity).me()
                store.updateUser(user)
                openMain()
            } catch (error: ApiException) {
                if (error.statusCode == 401) {
                    store.clear()
                    loginError.text = error.message
                    loginError.isVisible = true
                    focusFirstField()
                    checkServerStatus()
                } else {
                    showError(error.message ?: getString(R.string.error_generic))
                }
            } catch (error: Exception) {
                showError(error.message ?: getString(R.string.connection_error))
            } finally {
                setBusy(false, getString(R.string.login_button))
            }
        }
    }

    private fun attemptLogin() {
        if (busy) return
        hideKeyboard()
        loginError.isVisible = false
        val normalized = try {
            ApiClient.normalizeBaseUrl(serverInput.text.toString())
        } catch (error: IllegalArgumentException) {
            showError(getString(R.string.bad_url))
            return
        }
        val username = usernameInput.text.toString().trim()
        val password = passwordInput.text.toString()
        if (username.isBlank() || password.isBlank()) {
            showError("Udfyld brugernavn og adgangskode.")
            return
        }

        setBusy(true, getString(R.string.login_checking))
        lifecycleScope.launch {
            try {
                val result = ApiClient(normalized, context = this@LoginActivity).login(username, password)
                store.save(normalized, result.token, result.user)
                openMain()
            } catch (error: ApiException) {
                showError(error.message ?: getString(R.string.login_error))
            } catch (error: NetworkException) {
                showError(error.message ?: getString(R.string.connection_error))
            } catch (error: Exception) {
                showError(
                    if (error.message?.contains("Keystore", ignoreCase = true) == true) {
                        getString(R.string.token_secure_error)
                    } else {
                        error.message ?: getString(R.string.login_error)
                    },
                )
            } finally {
                setBusy(false, getString(R.string.login_button))
            }
        }
    }

    private fun setBusy(value: Boolean, label: String) {
        busy = value
        progress.isVisible = value
        loginButton.isEnabled = !value
        loginButton.text = label
        serverInput.isEnabled = !value
        usernameInput.isEnabled = !value
        passwordInput.isEnabled = !value
    }

    private fun showError(message: String) {
        loginError.text = message
        loginError.isVisible = true
    }

    private fun openMain() {
        startActivity(
            Intent(this, MainActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
            },
        )
        finish()
    }

    private fun hideKeyboard() {
        val manager = getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager
        manager.hideSoftInputFromWindow(passwordInput.windowToken, 0)
    }
}
