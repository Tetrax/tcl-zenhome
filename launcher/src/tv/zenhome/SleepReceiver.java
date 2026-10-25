package tv.zenhome;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Handler;
import android.os.Looper;
import android.widget.Toast;

import java.util.Locale;

/** Alarmes du minuteur de veille (voir Sleep). Non exporte : seul notre PendingIntent l'atteint. */
public class SleepReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(final Context c, Intent i) {
        if (i == null || i.getAction() == null) return;
        final boolean fr = "fr".equals(Locale.getDefault().getLanguage());
        SharedPreferences p = c.getSharedPreferences("zenhome", Context.MODE_PRIVATE);
        long saved = p.getLong("sleep_at", 0), fired = i.getLongExtra(Sleep.EXTRA_AT, -1), now = System.currentTimeMillis();
        if (Sleep.A_WARN.equals(i.getAction())) {
            if (SleepRules.warnDue(saved, fired, now)) Toast.makeText(c, SleepRules.warnText(fr), Toast.LENGTH_LONG).show();
        } else if (Sleep.A_SLEEP.equals(i.getAction()) && SleepRules.due(saved, fired, now)) {
            p.edit().remove("sleep_at").remove("sleep_min").apply();
            // le service des touches peut etre en train de se relier (processus relance par l'alarme) : quelques essais
            final PendingResult pr = goAsync();
            final Handler h = new Handler(Looper.getMainLooper());
            h.post(new Runnable() {
                int tries = 0;
                public void run() {
                    if (KeyService.sleepNow()) pr.finish();
                    else if (++tries < 6) h.postDelayed(this, 1000);
                    else { Toast.makeText(c, SleepRules.failText(fr), Toast.LENGTH_LONG).show(); pr.finish(); }
                }
            });
        }
    }
}
