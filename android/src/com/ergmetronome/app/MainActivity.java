package com.ergmetronome.app;

import android.app.NativeActivity;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowInsetsController;
import android.view.WindowManager;

public class MainActivity extends NativeActivity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        Window window = getWindow();
        // Keep the screen awake for the entire rowing workout.
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        int bgColor = Color.rgb(16, 19, 24);
        window.getDecorView().setBackgroundColor(bgColor);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            WindowInsetsController controller = window.getInsetsController();
            if (controller != null) {
                // Ensure light (white) status/nav icons on our dark #101318 background.
                controller.setSystemBarsAppearance(
                    0,
                    WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                        | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
                );
            }
        } else {
            @SuppressWarnings("deprecation")
            int flags = window.getDecorView().getSystemUiVisibility()
                & ~View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
            window.getDecorView().setSystemUiVisibility(flags);
        }

        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.VANILLA_ICE_CREAM) {
            @SuppressWarnings("deprecation")
            int unused = 0;
            window.setStatusBarColor(bgColor);
            window.setNavigationBarColor(bgColor);
        }
    }
}
