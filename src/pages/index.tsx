import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import Heading from '@theme/Heading';

import styles from './index.module.css';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <div className={styles.badge}>Backend / Cloud Native / JVM / Linux / LLM</div>
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <p className={styles.lead}>
          这里主要写后端工程、云原生基础设施、JVM 性能调优、Linux 运维排障，
          以及 LLM 应用落地里的架构设计、工程经验和踩坑复盘。
        </p>
        <div className={styles.buttons}>
          <Link className="button button--secondary button--lg" to="/blog">
            看最新文章
          </Link>
          <Link className="button button--outline button--lg" to="/docs/kubernetes-index">
            直接看 K8s 专题
          </Link>
          <Link className="button button--outline button--lg" to="/docs/intro">
            看专题文档
          </Link>
        </div>
      </div>
    </header>
  );
}

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={`${siteConfig.title}`}
      description="后端、云原生、JVM、Linux 与 LLM 的技术文章、实践总结和排障记录">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
