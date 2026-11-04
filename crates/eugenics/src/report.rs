//! Отчёт об обучении клуба.
//!
//! Отчёт пишется в конце прогона: по поколениям видно, растёт ли лучший
//! счёт, а по элите — какие бойцы оказались сильнее.

use std::{io, path::Path};

use super::Club;

/// Имя файла отчёта
pub const REPORT_NAME: &str = "report.md";

/// Пишет отчёт об обучении
pub fn write_report(dir: &Path, club: &Club) -> io::Result<()> {
    let text = build_report(dir, club);

    std::fs::write(dir.join(REPORT_NAME), text)
}

/// Собирает текст отчёта
fn build_report(dir: &Path, club: &Club) -> String {
    let mut text = String::new();

    let fights: u32 = club.history.iter().map(|report| report.fights).sum();
    let seconds: f32 = club.history.iter().map(|report| report.seconds).sum();

    text.push_str(&format!("# Отчёт обучения клуба «{}»\n\n", club.name));
    text.push_str(&format!("- элита: {}\n", club.config.elite));
    text.push_str(&format!("- наследников за поколение: {}\n", club.config.offspring));
    text.push_str(&format!(
        "- бойцов в поколении: {}\n",
        club.config.population_size()
    ));
    text.push_str(&format!("- поколений: {}\n", club.history.len()));
    text.push_str(&format!("- боёв всего: {}\n", fights));
    text.push_str(&format!("- время обучения: {:.1} с\n", seconds));
    text.push_str(&format!("- ширина сети: {}\n", club.config.hidden));
    text.push_str(&format!("- базовое зерно: {}\n\n", club.config.seed));

    text.push_str("## Расценка очков\n\n");
    text.push_str("| Исход боя | Победителю | Проигравшему |\n");
    text.push_str("|---|---|---|\n");
    text.push_str("| Ничья | −1 | −1 |\n");
    text.push_str("| Выход за ринг | +1 | −1 |\n");
    text.push_str("| Выдохся от энергии | +2 | −2 |\n");
    text.push_str("| Выдохся от удара | +4 | −4 |\n\n");

    text.push_str("## Динамика по поколениям\n\n");
    text.push_str(
        "| Поколение | Боёв | Время, с | Ср. бой, тиков | Лучший счёт | Ср. счёт | Худший счёт | Ничьи | Из кольца | Нокаут (энергия) | Нокаут (удар) | Разнообразие, % | Сила, % |\n",
    );
    text.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");

    for report in &club.history {
        text.push_str(&format!(
            "| {} | {} | {:.1} | {:.0} | {} | {:.1} | {} | {} | {} | {} | {} | {:.1} | {:.1} |\n",
            report.generation,
            report.fights,
            report.seconds,
            report.avg_ticks,
            report.best_score,
            report.mean_score,
            report.worst_score,
            report.draws,
            report.out_of_ring,
            report.exhausted,
            report.defeated,
            report.diversity * 100.,
            report.strength * 100.,
        ));
    }

    text.push_str(
        "\n«Сила» - доля побед элиты поколения над замороженной элитой первого\n\
         поколения: половина означает, что поколение не сильнее первого.\n",
    );

    text.push_str("\n## Текущая элита\n\n");
    text.push_str(
        "| Боец | Очки | Победы | Ничьи | Поражения | Нокаут (удар) | Нокаут (энергия) | Из кольца | Удар+ | Удар− | Ср. бой, тиков |\n",
    );
    text.push_str("|---|---|---|---|---|---|---|---|---|---|---|\n");

    for elite in &club.elites {
        let avg_ticks = if elite.stats.fights == 0 {
            0.
        } else {
            elite.stats.ticks as f32 / elite.stats.fights as f32
        };

        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} |\n",
            elite.name,
            elite.score,
            elite.stats.wins,
            elite.stats.draws,
            elite.stats.losses,
            elite.stats.wins_by_defeat,
            elite.stats.wins_by_exhaustion,
            elite.stats.wins_by_ring,
            elite.stats.hits_dealt,
            elite.stats.hits_taken,
            avg_ticks,
        ));
    }

    text.push_str("\n## Выводы\n\n");
    text.push_str(&conclusions(club));

    if club.elites.len() >= 2 {
        let first = &club.elites[0];
        let second = &club.elites[1];

        text.push_str("\n## Посмотреть бой лучших\n\n");
        text.push_str("```sh\n");
        text.push_str(&format!(
            "cargo run -- fight --red={} --blue={}\n",
            dir.join(format!("epoch_{:02}/{}.json", club.generation, first.name)).display(),
            dir.join(format!("epoch_{:02}/{}.json", club.generation, second.name)).display()
        ));
        text.push_str("```\n");
    }

    text
}

/// Выводы по обучению
fn conclusions(club: &Club) -> String {
    let mut text = String::new();

    let Some(first) = club.history.first() else {
        return "- поколений не было, выводов нет\n".to_string();
    };

    let Some(last) = club.history.last() else {
        return "- поколений не было, выводов нет\n".to_string();
    };

    if last.best_score > first.best_score {
        text.push_str(&format!(
            "- лучший счёт вырос с {} до {}\n",
            first.best_score, last.best_score
        ));
    } else {
        text.push_str(&format!(
            "- лучший счёт почти не изменился: {} в начале и {} в конце\n",
            first.best_score, last.best_score
        ));
    }

    // Очки в сумме у всех бойцов поколения всегда одинаковы, поэтому о самом
    // прогрессе говорит число исходов: нокаут от удара стоит +4 очка
    if last.defeated > first.defeated {
        text.push_str(&format!(
            "- нокаутов от удара за поколение выросло с {} до {}: бойцы научились побеждать нокаутом\n",
            first.defeated, last.defeated
        ));
    }

    if last.out_of_ring < first.out_of_ring {
        text.push_str(&format!(
            "- выходов из кольца стало меньше: {} против {} в начале\n",
            last.out_of_ring, first.out_of_ring
        ));
    }

    let defeated: u32 = club.history.iter().map(|report| report.defeated).sum();
    let exhausted: u32 = club.history.iter().map(|report| report.exhausted).sum();
    let draws: u32 = club.history.iter().map(|report| report.draws).sum();
    let out_of_ring: u32 = club.history.iter().map(|report| report.out_of_ring).sum();

    if defeated == 0 {
        text.push_str("- ни одного нокаута от удара: выигрывать очками нечем, отбор идёт по выходу из кольца\n");
    } else {
        text.push_str(&format!(
            "- нокаутов от удара за всё обучение: {} (это самый дорогой исход: +4 очка победителю)\n",
            defeated
        ));
    }

    if draws > 0 && draws * 2 > draws + exhausted + out_of_ring {
        text.push_str("- больше половины боёв закончились ничьёй: сигнала для отбора почти нет\n");
    }

    if last.diversity < 0.01 {
        text.push_str("- элита выродилась: геномы бойцов почти не различаются\n");
    } else {
        text.push_str(&format!(
            "- разнообразие элиты: {:.1}% различающихся генов\n",
            last.diversity * 100.
        ));
    }

    text.push_str(&format!(
        "- исходы за всё обучение: нокаут от удара {}, нокаут от энергии {}, из кольца {}, ничьи {}\n",
        defeated, exhausted, out_of_ring, draws
    ));

    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Club, GaConfig};

    fn club() -> Club {
        let config = GaConfig {
            club: "club_test".to_string(),
            elite: 2,
            offspring: 2,
            generations: 1,
            repeat: 1,
            hidden: 4,
            mutation_rate: 0.1,
            mutation_scale: 0.3,
            crossover_rate: 0.5,
            tournament: 3,
            scoring: crate::scoring::Scoring::Classic,
            benchmark_repeat: 1,
            seed: 1,
        };

        let mut club = Club::new(config);

        club.generation = 1;
        club.history.push(crate::GenerationReport {
            generation: 1,
            fights: 6,
            seconds: 0.1,
            avg_ticks: 100.,
            best_score: 1010,
            mean_score: 1000.,
            worst_score: 990,
            draws: 1,
            out_of_ring: 3,
            exhausted: 1,
            defeated: 1,
            diversity: 0.5,
            strength: 0.6,
            strength_fights: 64,
        });

        club
    }

    /// Отчёт содержит все таблицы
    #[test]
    fn report_has_all_sections() {
        let text = build_report(Path::new("club_eugenics"), &club());

        assert!(text.contains("# Отчёт обучения клуба"));
        assert!(text.contains("## Расценка очков"));
        assert!(text.contains("## Динамика по поколениям"));
        assert!(text.contains("## Текущая элита"));
        assert!(text.contains("## Выводы"));
    }

    /// В отчёте видно расценку очков
    #[test]
    fn report_shows_scoring_rules() {
        let text = build_report(Path::new("club_eugenics"), &club());

        assert!(text.contains("| Выдохся от удара | +4 | −4 |"));
        assert!(text.contains("| Ничья | −1 | −1 |"));
    }
}
